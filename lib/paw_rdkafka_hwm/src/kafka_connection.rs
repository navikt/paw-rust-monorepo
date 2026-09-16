use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::{error::KafkaError, kafka_config::KafkaConfig};
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler, rebalance_message::RebalanceMessage,
};

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<HwmRebalanceHandler>, KafkaError> {
    create_kafka_consumer_inner(app_state, pg_pool, kafka_config, topics, None)
}
pub fn create_kafka_consumer_with_sender(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
    sender: UnboundedSender<RebalanceMessage>,
) -> Result<StreamConsumer<HwmRebalanceHandler>, KafkaError> {
    create_kafka_consumer_inner(app_state, pg_pool, kafka_config, topics, Some(sender))
}

fn create_kafka_consumer_inner(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
    sender: Option<UnboundedSender<RebalanceMessage>>,
) -> Result<StreamConsumer<HwmRebalanceHandler>, KafkaError> {
    let config = kafka_config.rdkafka_client_config()?;
    let context = HwmRebalanceHandler {
        pg_pool,
        app_state,
        version: *kafka_config.hwm_version,
        sender,
    };
    let consumer: StreamConsumer<HwmRebalanceHandler> = config
        .create_with_context(context)
        .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?;
    consumer
        .subscribe(topics)
        .map_err(|e| KafkaError::Subscribe(e.to_string()))?;
    Ok(consumer)
}
