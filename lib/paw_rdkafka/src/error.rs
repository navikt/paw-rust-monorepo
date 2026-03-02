use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum KafkaError {
    #[error("Could not parse Kafka config: {0}")]
    Config(String),
    #[error("Could not create Kafka consumer: {0}")]
    CreateConsumer(String),
    #[error("Received unexpected Kafka message: {0}")]
    UnexpectedMessage(String),
}
