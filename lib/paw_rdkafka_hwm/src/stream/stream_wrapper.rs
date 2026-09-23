use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use chrono::DateTime;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use prometheus::{CounterVec, Gauge, register_counter_vec, register_gauge};
use rdkafka::Message;
use rdkafka::{consumer::StreamConsumer, message::OwnedMessage};
use sqlx::PgPool;
use tokio::sync::mpsc::{
    UnboundedReceiver, error::TryRecvError::Disconnected, error::TryRecvError::Empty,
};
use tokio::time::timeout_at;
use tracing::{Span, instrument};

use crate::hwm_functions::get_hwm;
use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler,
    rebalance_message::{RebalanceMessage, TopicPartition},
};
use crate::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use crate::stream::queue_handler::QueueHandler;
use crate::stream::queue_handler_list::{MessageOrKey, ensure_queue_and_push, push_if_assigned};

pub struct PawKafkaConsumerStream {
    /// Receiver for rebalance events from the rebalancer.
    receiver: UnboundedReceiver<RebalanceMessage>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    /// Currently active queues for each assigned topic partition.
    queues: Vec<QueueHandler>,
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

impl PawKafkaStream for PawKafkaConsumerStream {
    #[tracing::instrument(
        skip(self),
        name = "paw_kafka_stream.receive",
        fields(
            empty_before_load,
            empty_after_load,
            topic,
            partition,
            offset,
            timestamp,
            back_in_time_ms
        )
    )]
    /// Receives the next message from the stream, handling rebalance events
    /// and loading messages from the internal queues.
    /// Consumes self and returns it self if safe to continue receiving messages,
    /// or an error if the stream is disconnected or failed.
    async fn receive(mut self) -> Result<(Self, Option<OwnedMessage>), StreamError> {
        self.drain_and_rebalance().await?;
        let empty_before_load = self.queues.iter().filter(|q| q.is_empty()).count();
        load(&mut self.queues, self.max_idle).await?;
        let empty_after_load = self.queues.iter().filter(|q| q.is_empty()).count();
        // A queue that is empty but still inside its grace period may yet
        // deliver an older message, so emitting now would move the stream
        // clock past it.
        let within_grace = self
            .queues
            .iter()
            .filter_map(|q| q.empty_for())
            .any(|idle_for| idle_for < self.max_idle);
        if within_grace {
            Span::current().record("empty_before_load", empty_before_load as u64);
            Span::current().record("empty_after_load", empty_after_load as u64);
            Span::current().record("topic", "waiting");
            Span::current().record("partition", -1);
            Span::current().record("offset", -1);
            Span::current().record("timestamp", "waiting");
            return Ok((self, None));
        }
        let result = self
            .queues
            .iter_mut()
            .filter(|q| !q.is_empty())
            .min_by_key(|q| q.timestamp())
            .and_then(|q| q.take_head());
        Span::current().record("empty_before_load", empty_before_load as u64);
        Span::current().record("empty_after_load", empty_after_load as u64);
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
            let mut back_in_time = false;
            self.stream_times
                .entry(msg.partition())
                .and_modify(|ts| {
                    let new_ts = msg.timestamp().to_millis().unwrap_or(-1);
                    if new_ts >= *ts {
                        *ts = new_ts;
                    } else {
                        back_in_time = true;
                        let back_in_time_ms = *ts - new_ts;
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
                .or_insert_with(|| msg.timestamp().to_millis().unwrap_or(-1));
            STREAM_WRAPPER_MESSAGES
                .with_label_values(&[if back_in_time { "true" } else { "false" }])
                .inc();
            STREAM_WRAPER_TIMESTAMP.set(
                msg.timestamp()
                    .to_millis()
                    .and_then(DateTime::from_timestamp_millis)
                    .map(|dt| dt.timestamp_millis() as f64)
                    .unwrap_or(0.0),
            );
        } else {
            Span::current().record("topic", "none");
            Span::current().record("partition", -1);
            Span::current().record("offset", -1);
            Span::current().record("timestamp", "none");
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
async fn load(queues: &mut Vec<QueueHandler>, max_idle: Duration) -> Result<(), StreamError> {
    let now = tokio::time::Instant::now();
    let deadline = queues
        .iter()
        .filter_map(|q| q.empty_for())
        .filter(|idle_for| *idle_for < max_idle)
        .map(|idle_for| now + (max_idle - idle_for))
        .max()
        .unwrap_or(now + max_idle);
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

impl PawKafkaConsumerStream {
    pub fn new(
        receiver: UnboundedReceiver<RebalanceMessage>,
        consumer: StreamConsumer<HwmRebalanceHandler>,
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
        fields(rebalance_events = rebalance_events.len() as u64)
    )]
    fn handle_rebalance_events(
        &mut self,
        rebalance_events: Vec<RebalanceMessage>,
    ) -> Result<(), StreamError> {
        let is_empty = rebalance_events.is_empty();
        for rebalance_event in rebalance_events {
            match rebalance_event {
                RebalanceMessage::NoOp => {}
                RebalanceMessage::Assigned { topic_partitions } => {
                    tracing::info!("Assigned topic partition queues: {:?}", topic_partitions);
                    for TopicPartition { topic, partition } in topic_partitions {
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
                                        QueueHandler::new(key, pt_queue, self.internal_buffer_size)
                                    })
                            },
                        );
                    }
                }
                RebalanceMessage::Revoked { topic_partitions } => {
                    self.queues.retain(|q| !topic_partitions.contains(&q.key));
                    tracing::info!("Deactivated topic partition queues: {:?}", topic_partitions);
                }
                RebalanceMessage::InternalReceiverDisconnected => {
                    tracing::info!("Rebalancer sent disconnected signal, closing stream");
                    return Err(StreamError::DisconnectedFromRebalancer);
                }
            }
        }
        if !is_empty {
            tracing::info!(
                "current partition queues: {:?}",
                self.queues.iter().map(|q| &q.key).collect::<Vec<_>>()
            );
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
                            messages.push(msg.detach());
                        }
                        None => messages.push(msg.detach()),
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
                        self.handle_rebalance_events(rebalance_events)?;
                        none_counter = self.main_consumer_none_treshold.saturating_sub(3);
                    }
                }
            }
        }
        messages.into_iter().for_each(|msg| {
            let topic = msg.topic().to_string();
            let outcome = if push_if_assigned(&mut self.queues, msg) {
                "queued"
            } else {
                "not_assigned"
            };
            MAIN_QUEUE_MESSAGES
                .with_label_values(&[&topic, outcome])
                .inc();
        });
        Ok(())
    }
}

#[tracing::instrument(skip(receiver), name = "paw_kafka_stream.get_rebalance_events")]
fn get_rebalance_events(
    receiver: &mut UnboundedReceiver<RebalanceMessage>,
) -> Vec<RebalanceMessage> {
    let mut messages: Vec<RebalanceMessage> = Vec::with_capacity(2);
    loop {
        match receiver.try_recv() {
            Ok(msg) => {
                messages.push(msg);
            }
            Err(Empty) => {
                break;
            }
            Err(Disconnected) => {
                messages.push(RebalanceMessage::InternalReceiverDisconnected);
                break;
            }
        }
    }
    messages
}

static STREAM_WRAPER_TIMESTAMP: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "paw_kafka_stream_timestamp",
        "The timestamp of the last message retrieved from the stream wrapper"
    )
    .expect("Failed to create gauge")
});

/// Every message handed to the caller of `receive()`, labelled by whether its
/// timestamp went backwards relative to the newest one already emitted for the
/// same partition. Summing over the label gives total throughput, so the share
/// of jumps is a ratio between two series of the same counter.
static STREAM_WRAPPER_MESSAGES: LazyLock<CounterVec> = LazyLock::new(|| {
    let counter = register_counter_vec!(
        "paw_kafka_stream_messages_total",
        "Messages delivered by the stream wrapper, by whether the timestamp went backwards",
        &["back_in_time"]
    )
    .expect("Failed to create counter");
    counter.with_label_values(&["true"]);
    counter.with_label_values(&["false"]);
    counter
});

/// Messages that arrived on the main consumer queue rather than on a split
/// partition queue. In steady state this should be close to zero; anything
/// else means partitions are assigned but not yet split. `outcome` is
/// `queued`, `below_hwm` or `not_assigned`, and summing over it gives every
/// message the main queue produced.
static MAIN_QUEUE_MESSAGES: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "paw_kafka_stream_main_queue_messages_total",
        "Messages collected from the main consumer queue, by topic and outcome",
        &["topic", "outcome"]
    )
    .expect("Failed to create counter")
});
