use crate::rebalance::topic_partition_update::TopicPartition;

use rdkafka::message::OwnedMessage;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamError {
    #[error("Internal stream map out of sync with assigned topic partitions")]
    InternalStreamNotFound,
    #[error("Failed to read record from kafka queue")]
    FailedToReadRecord(String),
    #[error("Received 'disconnected' signal from rebalancer module")]
    DisconnectedFromRebalancer,
    #[error("Logic error in stream, claims assigned not called, but queues are available!")]
    InternalLogicError,
    #[error("Failed to access db during hwm filtering")]
    HwmFilterDbError,
}

pub trait PawKafkaStream {
    fn receive(
        self,
    ) -> impl std::future::Future<Output = Result<(Self, Option<OwnedMessage>), StreamError>> + Send
    where
        Self: Sized + Send;

    fn assigned(&self) -> Vec<TopicPartition>;
}
