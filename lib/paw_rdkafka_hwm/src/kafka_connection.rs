use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::{error::KafkaError, kafka_config::KafkaConfig};
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler, topic_partition_update::TopicPartitionUpdate,
};

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<HwmRebalanceHandler>, KafkaError> {
    let context = HwmRebalanceHandler::new(pg_pool, app_state, *kafka_config.hwm_version);
    create_kafka_consumer_inner(kafka_config, topics, context)
}
pub fn create_kafka_consumer_with_sender(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
    sender: UnboundedSender<TopicPartitionUpdate>,
) -> Result<StreamConsumer<HwmRebalanceHandler>, KafkaError> {
    let context =
        HwmRebalanceHandler::new_with_sender(pg_pool, app_state, *kafka_config.hwm_version, sender);
    create_kafka_consumer_inner(kafka_config, topics, context)
}

fn create_kafka_consumer_inner(
    kafka_config: KafkaConfig,
    topics: &[&str],
    context: HwmRebalanceHandler,
) -> Result<StreamConsumer<HwmRebalanceHandler>, KafkaError> {
    let config = kafka_config.rdkafka_client_config()?;
    let consumer: StreamConsumer<HwmRebalanceHandler> = config
        .create_with_context(context)
        .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?;
    consumer
        .subscribe(topics)
        .map_err(|e| KafkaError::Subscribe(e.to_string()))?;
    Ok(consumer)
}
