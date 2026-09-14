use rdkafka::{Message, Timestamp};

use crate::stream::paw_kafka_stream::StreamError;

use crate::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;

use rdkafka::consumer::stream_consumer::StreamPartitionQueue;

use rdkafka::message::OwnedMessage;

use crate::rebalance::rebalance_message::TopicPartition;

pub(crate) struct QueueHandler {
    pub key: TopicPartition,
    pub head: Option<OwnedMessage>,
    pub(crate) rdkafka_stream: StreamPartitionQueue<HwmRebalanceHandler>,
}

impl QueueHandler {
    pub async fn update(&mut self) -> Result<(), StreamError> {
        if self.head.is_none() {
            match self.rdkafka_stream.recv().await {
                Ok(record) => {
                    self.head = Some(record.detach());
                    Ok(())
                }
                Err(e) => Err(StreamError::FailedToReadRecord(e.to_string())),
            }
        } else {
            Ok(())
        }
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        if self.head.is_some() {
            self.head.as_ref().map(|m| m.timestamp())
        } else {
            None
        }
    }
}
