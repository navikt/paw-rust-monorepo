#[derive(Debug, thiserror::Error)]
pub enum RebalanceError {
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),

    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),
}
