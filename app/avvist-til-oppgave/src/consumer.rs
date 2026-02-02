use health_and_monitoring::simple_app_state::AppState;
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::error::Error;
use std::sync::Arc;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<HwmRebalanceHandler>, Box<dyn Error>> {
    let hwm_version = kafka_config.hwm_version;
    let config = kafka_config.rdkafka_client_config()?;
    let context = HwmRebalanceHandler {
        pg_pool,
        app_state,
        version: hwm_version,
    };
    let consumer: StreamConsumer<HwmRebalanceHandler> = config.create_with_context(context)?;
    consumer.subscribe(topics)?;
    Ok(consumer)
}
