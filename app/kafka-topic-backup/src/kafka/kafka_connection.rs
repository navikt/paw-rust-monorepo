use std::{error::Error, sync::Arc};

use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;

use crate::kafka::config::ApplicationKafkaConfig;

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    app_config: ApplicationKafkaConfig,
    topics: &[&str],
    hwm_version: i16,
) -> Result<StreamConsumer<HwmRebalanceHandler>, Box<dyn Error>> {
    let config = app_config.rdkafka_config()?;
    let context = HwmRebalanceHandler { pg_pool, app_state, version: hwm_version };
    let consumer: StreamConsumer<HwmRebalanceHandler> = config.create_with_context(context)?;
    consumer.subscribe(topics)?;
    Ok(consumer)
}
