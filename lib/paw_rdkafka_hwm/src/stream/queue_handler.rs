use std::collections::VecDeque;

use paw_rdkafka::error::KafkaError;
use rdkafka::{Message, Timestamp};

use crate::stream::paw_kafka_stream::StreamError;

use crate::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;

use rdkafka::consumer::stream_consumer::StreamPartitionQueue;

use rdkafka::message::OwnedMessage;

use crate::rebalance::rebalance_message::TopicPartition;

pub struct QueueHandler {
    pub key: TopicPartition,
    head: VecDeque<OwnedMessage>,
    rdkafka_stream: StreamPartitionQueue<HwmRebalanceHandler>,
    internal_buffer_size: usize,
}

impl QueueHandler {
    pub fn new(
        key: TopicPartition,
        rdkafka_stream: StreamPartitionQueue<HwmRebalanceHandler>,
        internal_buffer_size: usize,
    ) -> Self {
        Self {
            key,
            head: VecDeque::new(),
            rdkafka_stream,
            internal_buffer_size,
        }
    }

    pub fn key(&self) -> &TopicPartition {
        &self.key
    }

    pub fn take_head(&mut self) -> Option<OwnedMessage> {
        self.head.pop_front()
    }
    pub async fn update(&mut self) -> Result<(), StreamError> {
        if self.head.is_empty() {
            while self.head.len() < self.internal_buffer_size {
                match self.rdkafka_stream.recv().await {
                    Ok(record) => {
                        self.head.push_back(record.detach());
                    }
                    Err(e) => return Err(StreamError::FailedToReadRecord(e.to_string())),
                }
            }
        }
        Ok(())
    }

    pub fn add_message(&mut self, msg: OwnedMessage) -> Result<usize, KafkaError> {
        let topic = msg.topic().to_string();
        let partition = msg.partition();
        if self.key.topic != topic || self.key.partition != partition {
            return Err(KafkaError::UnexpectedMessage(format!(
                "Message topic/partition ({}/{}) does not match queue key ({}/{})",
                topic, partition, self.key.topic, self.key.partition
            )));
        }
        self.head.push_back(msg);
        Ok(self.head.len())
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        self.head.front().map(|msg| msg.timestamp())
    }
}
