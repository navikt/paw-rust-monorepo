use crate::kafka::hendelse::Hendelse;
use crate::kafka::kafka_message::KafkaMessage;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use serde_json::Value;
use sqlx::PgPool;
use std::error::Error;
use std::sync::Arc;

const AVVIST_HENDELSE_TYPE: &str = "intern.v1.avvist";
const AARSAK_UNDER_18: &str = "Er under 18 år";

pub async fn process_hendelse(
    hendelselogg_consumer: StreamConsumer<HwmRebalanceHandler>,
    pg_pool: PgPool,
    app_state: Arc<AppState>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let borrowed_message = hendelselogg_consumer.recv().await?;
        let kafka_message = KafkaMessage::from_borrowed_message(borrowed_message)?;
        let json: Value = serde_json::from_slice(&kafka_message.payload)?;
        let hendelse_type = json["hendelseType"].as_str().unwrap_or_default();
        let aarsak = json["metadata"]["aarsak"].as_str().unwrap_or_default();

        match hendelse_type == AVVIST_HENDELSE_TYPE && aarsak == AARSAK_UNDER_18 {
            true => {
                let hendelse: Hendelse = serde_json::from_value(json)?;
                log::info!("Prosesserer avvist hendelse for arbeidssøker");
            }
            false => { /* Gå videre */ },
        }
    }
    Ok(())
}
