use crate::logic::process::message_processor::OversiktMessageProcessor;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::hwm_message_processor::hwm_process_message;
use paw_rdkafka_hwm::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use paw_rust_base::error::ServerError;
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> anyhow::Result<StreamConsumer<HwmRebalanceHandler>> {
    let hwm_version = *kafka_config.hwm_version;
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

pub fn kafka_consumer_task(
    pg_pool: PgPool,
    hwm_version: i16,
    consumer: StreamConsumer<HwmRebalanceHandler>,
    processor: OversiktMessageProcessor,
) -> JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move {
        loop {
            let msg = consumer.recv().await?.detach();
            hwm_process_message(hwm_version, pg_pool.clone(), &msg, &processor)
                .await
                .map_err(|e| ServerError::InternalProcessTerminated {
                    process: "KafkaConsumer".to_string(),
                    message: e.to_string(),
                })?;
        }
    })
}
