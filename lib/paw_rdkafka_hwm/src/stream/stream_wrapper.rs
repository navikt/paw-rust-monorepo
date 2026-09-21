use std::sync::Arc;
use std::time::Duration;

use chrono::DateTime;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use rdkafka::Message;
use rdkafka::{consumer::StreamConsumer, message::OwnedMessage};
use tokio::sync::mpsc::{
    UnboundedReceiver, error::TryRecvError::Disconnected, error::TryRecvError::Empty,
};
use tokio::time::timeout_at;
use tracing::{Span, instrument};

use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler,
    rebalance_message::{RebalanceMessage, TopicPartition},
};
use crate::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use crate::stream::queue_handler::QueueHandler;
use crate::stream::queue_handler_list::{MessageOrKey, ensure_queue_and_push};

pub struct PawKafkaConsumerStream {
    receiver: UnboundedReceiver<RebalanceMessage>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    queues: Vec<QueueHandler>,
    timeout: Duration,
    internal_buffer_size: usize,
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
            timestamp
        )
    )]
    async fn receive(mut self) -> Result<(Self, Option<OwnedMessage>), StreamError> {
        self.drain_main_consumer().await?;
        let rebalance_events = get_rebalance_events(&mut self.receiver);
        self.handle_rebalance_events(rebalance_events)?;
        let empty_before_load = self.queues.iter().filter(|q| q.is_empty()).count();
        load(&mut self.queues, self.timeout).await?;
        let empty_after_load = self.queues.iter().filter(|q| q.is_empty()).count();
        let result = self
            .queues
            .iter_mut()
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
        } else {
            tracing::info!("No messages available in any queue");
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
async fn load(queues: &mut Vec<QueueHandler>, timeout: Duration) -> Result<(), StreamError> {
    let mut updates = FuturesUnordered::new();
    for queue in queues {
        updates.push(queue.update());
    }
    let deadline = tokio::time::Instant::now() + timeout;
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
        timeout: Duration,
        internal_buffer_size: usize,
    ) -> Self {
        Self {
            receiver,
            consumer: consumer.into(),
            queues: Vec::new(),
            timeout,
            internal_buffer_size,
        }
    }

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

    async fn drain_main_consumer(&mut self) -> Result<(), StreamError> {
        while let Some(res) = self.consumer.recv().now_or_never() {
            match res {
                Ok(msg) => {
                    tracing::info!(
                        "Received early message from topic {} partition {} offset {}",
                        msg.topic(),
                        msg.partition(),
                        msg.offset()
                    );
                    let msg = msg.detach();
                    ensure_queue_and_push(&mut self.queues, MessageOrKey::Message(msg), |tp| {
                        self.consumer
                            .split_partition_queue(&tp.topic, tp.partition)
                            .map(|pt_queue| {
                                QueueHandler::new(tp, pt_queue, self.internal_buffer_size)
                            })
                    });
                }
                Err(e) => {
                    return Err(StreamError::FailedToReadRecord(e.to_string()));
                }
            }
        }
        Ok(())
    }
}
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
