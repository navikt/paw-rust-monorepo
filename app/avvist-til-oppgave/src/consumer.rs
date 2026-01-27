use crate::get_kafka_config::ApplicationKafkaConfig;
use crate::kafka_hwm::HwmRebalanceHandler;
use health_and_monitoring::simple_app_state::AppState;
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::error::Error;
use std::sync::Arc;

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    app_config: ApplicationKafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<HwmRebalanceHandler>, Box<dyn Error>> {
    let hwm_version = app_config.hwm_version;
    let config = app_config.rdkafka_config()?;
    let context = HwmRebalanceHandler {
        pg_pool,
        app_state,
        version: hwm_version,
    };
    let consumer: StreamConsumer<HwmRebalanceHandler> = config.create_with_context(context)?;
    consumer.subscribe(topics)?;
    Ok(consumer)
}
