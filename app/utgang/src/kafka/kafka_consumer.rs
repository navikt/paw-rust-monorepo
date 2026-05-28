use std::{error::Error, sync::Arc};

use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::rebalance::rebalance_handler::RebalanceHandler;
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<RebalanceHandler>, Box<dyn Error>> {
    let config = kafka_config.rdkafka_client_config()?;
    let context = RebalanceHandler {
        version: *kafka_config.hwm_version,
        pg_pool,
        app_state,
    };
    let consumer: StreamConsumer<RebalanceHandler> = config.create_with_context(context)?;
    consumer.subscribe(topics)?;
    Ok(consumer)
}
