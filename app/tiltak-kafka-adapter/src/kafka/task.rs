use crate::logic::metrics::kafka::register_message_metrics;
use crate::logic::process::root::RootProcessor;
use paw_kafka::hwm::hwm_message_processor::hwm_process_message;
use paw_kafka::hwm::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use paw_rust_base::error::ServerError;
use rdkafka::consumer::StreamConsumer;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub fn kafka_consumer_task(
    hwm_version: i16,
    pg_pool: PgPool,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    processor: RootProcessor,
) -> JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move {
        loop {
            let message = consumer.recv().await?.detach();
            register_message_metrics(&message);
            hwm_process_message(hwm_version, pg_pool.clone(), &message, &processor)
                .await
                .map_err(|e| ServerError::InternalProcessTerminated {
                    process: "KafkaConsumer".to_string(),
                    message: e.to_string(),
                })?;
        }
    })
}
