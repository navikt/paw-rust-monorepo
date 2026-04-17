use crate::message_processor::VeilederOppgaveMessageProcessor;
use anyhow::Result;
use paw_rdkafka_hwm::hwm_message_processor::hwm_process_message;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use paw_rust_base::error::ServerError;
use rdkafka::consumer::StreamConsumer;
use sqlx::PgPool;
use tokio::task::JoinHandle;

pub fn spawn_kafka_consumer_task(
    consumer: StreamConsumer<HwmRebalanceHandler>,
    hwm_version: i16,
    pg_pool: PgPool,
    processor: VeilederOppgaveMessageProcessor,
) -> JoinHandle<Result<()>> {
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
