use anyhow::Result;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::kafka_config::KafkaConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::sync::Arc;
use crate::rebalance::rebalance_handler::RebalanceHandler;

pub fn create_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<RebalanceHandler>> {
    let hwm_version = *kafka_config.hwm_version;
    let config = kafka_config.rdkafka_client_config()?;
    let context = RebalanceHandler {
        pg_pool,
        app_state,
        version: hwm_version,
    };
    let consumer: StreamConsumer<RebalanceHandler> = config.create_with_context(context)?;
    consumer.subscribe(topics)?;
    Ok(consumer)
}
