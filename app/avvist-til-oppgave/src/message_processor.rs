use crate::config::ApplicationConfig;
use crate::process_hendelselogg_message::process_hendelselogg_message;
use crate::process_oppgavehendelse_message::process_oppgavehendelse_message;
use anyhow::Result;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::error::KafkaError;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use rdkafka::message::Message;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn start_kafka_consumer_loop(
    hendelselogg_consumer: StreamConsumer<HwmRebalanceHandler>,
    pg_pool: PgPool,
    _app_state: Arc<AppState>,
    app_config: &ApplicationConfig,
) -> Result<()> {
    log::info!("Starting processing loop");
    let topic_hendelseslogg = app_config.topic_hendelseslogg.as_str();
    let topic_oppgavehendelse = app_config.topic_oppgavehendelse.as_str();

    loop {
        let kafka_message = hendelselogg_consumer.recv().await?.detach();
        tracing::info!(
            "Mottok Kafka-melding på topic: {}, partiton {}, offset: {}",
            kafka_message.topic(),
            kafka_message.partition(),
            kafka_message.offset()
        );

        let topic = kafka_message.topic();
        if topic == topic_hendelseslogg {
            process_hendelselogg_message(&kafka_message, app_config, pg_pool.clone()).await?;
        } else if topic == topic_oppgavehendelse {
            process_oppgavehendelse_message(&kafka_message, app_config, pg_pool.clone()).await?;
        } else {
            return Err(KafkaError::UnexpectedMessage(format!(
                "Mottok melding fra uventet topic {topic}"
            ))
            .into());
        }
    }
}
