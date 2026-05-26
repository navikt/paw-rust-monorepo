#[derive(Debug, thiserror::Error)]
pub(super) enum RebalanceError {
    #[error("Database-feil: {0}")]
    Database(#[from] anyhow::Error),

    #[error("Kafka-feil: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),
}
