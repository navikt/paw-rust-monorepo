use std::collections::HashSet;
use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use futures::stream::FuturesUnordered;
use rdkafka::{consumer::StreamConsumer, message::OwnedMessage};
use tokio::sync::mpsc::{
    UnboundedReceiver, error::TryRecvError::Disconnected, error::TryRecvError::Empty,
};
use tokio::time::{Instant, timeout_at};

use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler,
    rebalance_message::{RebalanceMessage, TopicPartition},
};
use crate::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use crate::stream::queue_handler::QueueHandler;

struct PawKafkaConsumerStream {
    rebalancelistener_timeout: Duration,
    receiver: UnboundedReceiver<RebalanceMessage>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    queues: Vec<QueueHandler>,
    assigned: HashSet<TopicPartition>,
    stream_time: Instant,
    timeout: Duration,
}

impl PawKafkaStream for PawKafkaConsumerStream {
    async fn receive(mut self) -> Result<(Self, Option<OwnedMessage>), StreamError> {
        let rebalance_events = get_all_messages(&mut self.receiver);
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
        self.assigned.iter().cloned().collect()
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
    fn new(
        rebalancelistener_timeout: Duration,
        receiver: UnboundedReceiver<RebalanceMessage>,
        consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
        timeout: Duration,
    ) -> Self {
        Self {
            rebalancelistener_timeout,
            receiver,
            consumer,
            queues: Vec::new(),
            assigned: HashSet::new(),
            stream_time: Instant::now(),
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
