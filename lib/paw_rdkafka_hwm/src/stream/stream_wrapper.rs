use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::stream::FuturesUnordered;
use rdkafka::{consumer::StreamConsumer, message::OwnedMessage};
use tokio::sync::mpsc::{
    UnboundedReceiver, error::TryRecvError::Disconnected, error::TryRecvError::Empty,
};
use tokio::time::timeout_at;

use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler,
    rebalance_message::{RebalanceMessage, TopicPartition},
};
use crate::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use crate::stream::queue_handler::QueueHandler;

pub struct PawKafkaConsumerStream {
    receiver: UnboundedReceiver<RebalanceMessage>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    queues: Vec<QueueHandler>,
    timeout: Duration,
}

impl PawKafkaStream for PawKafkaConsumerStream {
    #[tracing::instrument(skip(self), name = "PawKafkaConsumerStream::receive")]
    async fn receive(mut self) -> Result<(Self, Option<OwnedMessage>), StreamError> {
        let rebalance_events = get_all_messages(&mut self.receiver);
        if !rebalance_events.is_empty() {
            tracing::debug!("Received {} rebalance events", rebalance_events.len());
        }
        self.handle_rebalance_events(rebalance_events)?;
        load(&mut self.queues, self.timeout).await?;
        let mut index: usize = usize::MAX;
        let mut min_timestamp = i64::MAX;
        for (i, queue) in self.queues.iter().enumerate() {
            let Some(ts) = queue.timestamp() else {
                continue;
            };
            let candidate_timestamp = ts.to_millis().unwrap_or(i64::MAX);
            if candidate_timestamp < min_timestamp {
                min_timestamp = candidate_timestamp;
                index = i;
            }
        }
        if min_timestamp != i64::MAX {
            let queue = &mut self.queues[index];
            let record = queue.head.take();
            Ok((self, record))
        } else {
            Ok((self, None))
        }
    }

    fn assigned(&self) -> Vec<TopicPartition> {
        self.queues.iter().map(|q| q.key.clone()).collect()
    }
}

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
    ) -> Self {
        Self {
            receiver,
            consumer: consumer.into(),
            queues: Vec::new(),
            timeout,
        }
    }

    fn handle_rebalance_events(
        &mut self,
        rebalance_events: Vec<RebalanceMessage>,
    ) -> Result<(), StreamError> {
        for rebalance_event in rebalance_events {
            match rebalance_event {
                RebalanceMessage::NoOp => {}
                RebalanceMessage::Assigned { topic_partitions } => {
                    tracing::info!("Assigned topic partition queues: {:?}", topic_partitions);
                    self.queues.retain(|q| !topic_partitions.contains(&q.key));
                    for TopicPartition { topic, partition } in topic_partitions {
                        let stream = self
                            .consumer
                            .split_partition_queue(&topic, partition)
                            .ok_or(StreamError::InternalStreamNotFound)?;
                        tracing::info!("Activated topic partition queue: {}-{}", &topic, partition);
                        let queue_handler = QueueHandler {
                            key: TopicPartition { topic, partition },
                            head: None,
                            rdkafka_stream: stream,
                        };
                        self.queues.push(queue_handler);
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
        tracing::info!(
            "current partition queues: {:?}",
            self.queues.iter().map(|q| &q.key).collect::<Vec<_>>()
        );
        Ok(())
    }
}
fn get_all_messages(receiver: &mut UnboundedReceiver<RebalanceMessage>) -> Vec<RebalanceMessage> {
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
