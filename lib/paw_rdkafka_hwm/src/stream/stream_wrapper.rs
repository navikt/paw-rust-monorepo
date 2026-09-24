use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use chrono::DateTime;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use prometheus::{
    CounterVec, HistogramVec, exponential_buckets, register_counter_vec, register_histogram_vec,
};
use rdkafka::Message;
use rdkafka::{
    consumer::{StreamConsumer, stream_consumer::StreamPartitionQueue},
    message::OwnedMessage,
};
use sqlx::PgPool;
use tokio::sync::mpsc::{
    UnboundedReceiver, error::TryRecvError::Disconnected, error::TryRecvError::Empty,
};
use tokio::time::{Instant, timeout_at};
use tracing::{Span, instrument};

use crate::hwm_functions::get_hwm;
use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler,
    topic_partition_update::{TopicPartition, TopicPartitionUpdate},
};
use crate::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use crate::stream::queue_handler::{PartitionMessageSource, QueueHandler};
use crate::stream::queue_handler_list::{MessageOrKey, ensure_queue_and_push, push_if_assigned};

pub trait ConsumerMessageSource: Send + Sync {
    type PartitionSource: PartitionMessageSource;

    fn recv(&self) -> impl Future<Output = Result<OwnedMessage, StreamError>> + Send;

    fn split_partition_queue(
        self: &Arc<Self>,
        topic: &str,
        partition: i32,
    ) -> Option<Self::PartitionSource>;
}

impl ConsumerMessageSource for StreamConsumer<HwmRebalanceHandler> {
    type PartitionSource = StreamPartitionQueue<HwmRebalanceHandler>;

    async fn recv(&self) -> Result<OwnedMessage, StreamError> {
        StreamConsumer::recv(self)
            .await
            .map(|message| message.detach())
            .map_err(|error| StreamError::FailedToReadRecord(error.to_string()))
    }

    fn split_partition_queue(
        self: &Arc<Self>,
        topic: &str,
        partition: i32,
    ) -> Option<Self::PartitionSource> {
        StreamConsumer::split_partition_queue(self, topic, partition)
    }
}

pub type KafkaConsumerStream = PawKafkaConsumerStream<StreamConsumer<HwmRebalanceHandler>>;

pub struct PawKafkaConsumerStream<C: ConsumerMessageSource> {
    /// Receiver for rebalance events from the rebalancer.
    receiver: UnboundedReceiver<TopicPartitionUpdate>,
    consumer: Arc<C>,
    /// Currently active queues for each assigned topic partition.
    queues: Vec<QueueHandler<C::PartitionSource>>,
    /// Maximum time the internal buffer can be empty before we consider it idle and
    /// no longer wait for it to be filled.
    /// This is used to avoid slowing down the stream when a partition
    /// has no messages for a while.
    max_idle: Duration,
    /// Soft limit for the number of messages to buffer internally for each partition queue.
    internal_buffer_size: usize,
    stream_times: HashMap<i32, i64>,
    pg_pool: PgPool,
    hwm_version: i16,
    main_consumer_none_treshold: usize,
}

impl<C: ConsumerMessageSource> PawKafkaStream for PawKafkaConsumerStream<C> {
    #[tracing::instrument(
        skip(self),
        name = "paw_kafka_stream.receive",
        fields(topic, partition, offset, timestamp, back_in_time_ms)
    )]
    /// Receives the next message from the stream, handling rebalance events
    /// and loading messages from the internal queues.
    /// Consumes self and returns it self if safe to continue receiving messages,
    /// or an error if the stream is disconnected or failed.
    async fn receive(mut self) -> Result<(Self, Option<OwnedMessage>), StreamError> {
        self.drain_and_rebalance().await?;
        load(&mut self.queues, self.max_idle).await?;
        let stalled = self.queues.iter().filter(|q| q.has_stalled()).count();
        Span::current().record("stalled_queues", stalled as u64);
        if stalled > 0 {
            Span::current().record("topic", "waiting");
            Span::current().record("partition", -1);
            Span::current().record("offset", -1);
            Span::current().record("timestamp", "waiting");
            RECEIVE_RESULT.with_label_values(&["waiting"]).inc();
            return Ok((self, None));
        }
        let result = self
            .queues
            .iter_mut()
            .filter(|q| !q.is_empty())
            .min_by_key(|q| q.timestamp())
            .and_then(|q| q.take_head());
        if let Some(msg) = result.as_ref() {
            Span::current().record("topic", msg.topic());
            Span::current().record("partition", msg.partition());
            Span::current().record("offset", msg.offset());
            Span::current().record(
                "timestamp",
                msg.timestamp()
                    .to_millis()
                    .and_then(DateTime::from_timestamp_millis)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            let mut back_in_time_ms = 0;
            if let Some(new_ts) = msg.timestamp().to_millis() {
                self.stream_times
                    .entry(msg.partition())
                    .and_modify(|ts| {
                        if new_ts >= *ts {
                            *ts = new_ts;
                        } else {
                            back_in_time_ms = *ts - new_ts;
                            Span::current().record("back_in_time_ms", back_in_time_ms);
                            tracing::warn!(
                                back_in_time_ms,
                                stream_time_ms = *ts,
                                message_time_ms = new_ts,
                                kafka.topic = msg.topic(),
                                kafka.partition = msg.partition(),
                                kafka.offset = msg.offset(),
                                "kafka.back_in_time"
                            );
                        }
                    })
                    .or_insert(new_ts);
            }
            STREAM_WRAPPER_MESSAGES
                .with_label_values(&[if back_in_time_ms > 0 { "true" } else { "false" }])
                .inc();
            if back_in_time_ms > 0 {
                BACK_IN_TIME_MS
                    .with_label_values(&[msg.topic()])
                    .observe(back_in_time_ms as f64);
            }
            RECEIVE_RESULT.with_label_values(&["message"]).inc();
        } else {
            Span::current().record("topic", "none");
            Span::current().record("partition", -1);
            Span::current().record("offset", -1);
            Span::current().record("timestamp", "none");
            RECEIVE_RESULT.with_label_values(&["empty"]).inc();
            return Ok((self, None));
        };
        Ok((self, result))
    }

    fn assigned(&self) -> Vec<TopicPartition> {
        self.queues.iter().map(|q| q.key.clone()).collect()
    }
}

#[instrument(
    skip(queues),
    name = "paw_kafka_stream.load",
    fields(topic_partitions = queues.len() as u64)
)]
async fn load<S: PartitionMessageSource>(
    queues: &mut Vec<QueueHandler<S>>,
    max_idle: Duration,
) -> Result<(), StreamError> {
    let deadline = Instant::now() + max_idle;
    let mut updates = FuturesUnordered::new();
    for queue in queues {
        updates.push(queue.update());
    }
    while let Ok(Some(res)) = timeout_at(deadline, updates.next()).await {
        match res {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(())
}

impl<C: ConsumerMessageSource> PawKafkaConsumerStream<C> {
    pub fn new(
        receiver: UnboundedReceiver<TopicPartitionUpdate>,
        consumer: C,
        max_idle: Duration,
        internal_buffer_size: usize,
        pg_pool: PgPool,
        hwm_version: i16,
        main_consumer_none_treshold: usize,
    ) -> Self {
        Self {
            receiver,
            consumer: consumer.into(),
            queues: Vec::new(),
            max_idle,
            internal_buffer_size,
            stream_times: HashMap::new(),
            pg_pool,
            hwm_version,
            main_consumer_none_treshold,
        }
    }

    #[tracing::instrument(
        skip(self),
        name = "paw_kafka_stream.handle_rebalance_events",
        fields(topic_update_events = topic_update_events.len() as u64)
    )]
    fn handle_topic_update_events(
        &mut self,
        topic_update_events: Vec<TopicPartitionUpdate>,
    ) -> Result<(), StreamError> {
        for rebalance_event in topic_update_events {
            match rebalance_event {
                TopicPartitionUpdate::NoOp => {}
                TopicPartitionUpdate::Assigned {
                    topic_partition_hwms,
                } => {
                    tracing::info!(
                        "Assigned topic partition queues: {:?}",
                        topic_partition_hwms
                    );
                    for (TopicPartition { topic, partition }, hwm) in topic_partition_hwms {
                        ensure_queue_and_push(
                            &mut self.queues,
                            MessageOrKey::Key(TopicPartition {
                                topic: topic.clone(),
                                partition,
                            }),
                            |key| {
                                self.consumer
                                    .split_partition_queue(&key.topic, key.partition)
                                    .map(|pt_queue| {
                                        QueueHandler::new(
                                            key,
                                            pt_queue,
                                            self.internal_buffer_size,
                                            hwm,
                                        )
                                    })
                            },
                        )?;
                    }
                }
                TopicPartitionUpdate::Revoked { topic_partitions } => {
                    self.queues.retain(|q| !topic_partitions.contains(&q.key));
                    tracing::info!("Deactivated topic partition queues: {:?}", topic_partitions);
                }
                TopicPartitionUpdate::InternalReceiverDisconnected => {
                    tracing::info!("Rebalancer sent disconnected signal, closing stream");
                    return Err(StreamError::DisconnectedFromRebalancer);
                }
                TopicPartitionUpdate::HiOffsetUpdate {
                    topic_partition_offsets,
                } => {
                    topic_partition_offsets
                        .into_iter()
                        .for_each(|(tp, offsets)| {
                            if let Some(queue) =
                                self.queues.iter_mut().find(|queue| queue.key == tp)
                            {
                                queue.set_offsets(offsets);
                            }
                        });
                }
            }
        }
        Ok(())
    }

    async fn drain_and_rebalance(&mut self) -> Result<(), StreamError> {
        let mut messages = Vec::new();
        let mut none_counter = 0;
        while none_counter < self.main_consumer_none_treshold {
            match self.consumer.recv().now_or_never() {
                Some(Ok(msg)) => {
                    tracing::debug!(
                        "Received early message from topic {} partition {} offset {}",
                        msg.topic(),
                        msg.partition(),
                        msg.offset()
                    );
                    let mut tx = self
                        .pg_pool
                        .begin()
                        .await
                        .map_err(|_| StreamError::HwmFilterDbError)?;
                    let hwm = get_hwm(
                        &mut tx,
                        self.hwm_version,
                        msg.topic(),
                        msg.partition() as u16,
                    )
                    .await
                    .map_err(|_| StreamError::HwmFilterDbError)?;
                    match hwm {
                        Some(hwm) if msg.offset() > hwm => {
                            messages.push(msg);
                        }
                        None => messages.push(msg),
                        _ => {
                            MAIN_QUEUE_MESSAGES
                                .with_label_values(&[msg.topic(), "below_hwm"])
                                .inc();
                            tracing::trace!(
                                "[StreamWrapper] Message below HWM, topic {}, partition {}, offset {}, dropped",
                                msg.topic(),
                                msg.partition(),
                                msg.offset()
                            );
                        }
                    }
                }
                Some(Err(e)) => {
                    return Err(StreamError::FailedToReadRecord(e.to_string()));
                }
                None => {
                    let rebalance_events = get_rebalance_events(&mut self.receiver);
                    if rebalance_events.is_empty() {
                        none_counter += 1;
                    } else {
                        self.handle_topic_update_events(rebalance_events)?;
                        none_counter = self.main_consumer_none_treshold.saturating_sub(3);
                    }
                }
            }
        }
        for msg in messages {
            let topic = msg.topic().to_string();
            let outcome = if push_if_assigned(&mut self.queues, msg)? {
                "queued"
            } else {
                "not_assigned"
            };
            MAIN_QUEUE_MESSAGES
                .with_label_values(&[&topic, outcome])
                .inc();
        }
        Ok(())
    }
}

#[tracing::instrument(skip(receiver), name = "paw_kafka_stream.get_rebalance_events")]
fn get_rebalance_events(
    receiver: &mut UnboundedReceiver<TopicPartitionUpdate>,
) -> Vec<TopicPartitionUpdate> {
    let mut messages: Vec<TopicPartitionUpdate> = Vec::with_capacity(2);
    loop {
        match receiver.try_recv() {
            Ok(msg) => {
                messages.push(msg);
            }
            Err(Empty) => {
                break;
            }
            Err(Disconnected) => {
                messages.push(TopicPartitionUpdate::InternalReceiverDisconnected);
                break;
            }
        }
    }
    messages
}

static STREAM_WRAPPER_MESSAGES: LazyLock<CounterVec> = LazyLock::new(|| {
    let counter = register_counter_vec!(
        "paw_kafka_stream_messages_total",
        "Messages delivered by the stream wrapper, by whether the timestamp went backwards",
        &["back_in_time"]
    )
    .expect("Failed to create counter");
    // Registers the series so they read 0 before the first increment.
    counter.with_label_values(&["true"]);
    counter.with_label_values(&["false"]);
    counter
});

static RECEIVE_RESULT: LazyLock<CounterVec> = LazyLock::new(|| {
    let counter = register_counter_vec!(
        "paw_kafka_stream_receive_total",
        "Calls to the stream wrapper receive(), by result",
        &["result"]
    )
    .expect("Failed to create counter");
    // Registers the series so they read 0 before the first increment.
    for result in ["message", "waiting", "empty", "error"] {
        counter.with_label_values(&[result]);
    }
    counter
});

static BACK_IN_TIME_MS: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "paw_kafka_stream_back_in_time_ms",
        "Size of backward timestamp jumps in milliseconds, by topic",
        &["topic"],
        exponential_buckets(1.0, 10.0, 9).expect("Failed to create buckets")
    )
    .expect("Failed to create histogram")
});

static MAIN_QUEUE_MESSAGES: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "paw_kafka_stream_main_queue_messages_total",
        "Messages collected from the main consumer queue, by topic and outcome",
        &["topic", "outcome"]
    )
    .expect("Failed to create counter")
});
