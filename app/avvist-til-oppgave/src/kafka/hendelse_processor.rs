use crate::kafka::hendelse::Hendelse;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use rdkafka::message::Message;
use rdkafka::message::OwnedMessage;
use serde_json::Value;
use sqlx::PgPool;
use std::error::Error;
use std::sync::Arc;

const AVVIST_HENDELSE_TYPE: &str = "intern.v1.avvist";
const AARSAK_UNDER_18: &str = "Er under 18 år";

pub async fn start_processing_loop(
    hendelselogg_consumer: StreamConsumer<HwmRebalanceHandler>,
    pg_pool: PgPool,
    app_state: Arc<AppState>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let kafka_message = hendelselogg_consumer.recv().await?.detach();
        process_hendelse(&kafka_message, pg_pool.clone()).await?;
    }
}

async fn process_hendelse(
    kafka_message: &OwnedMessage,
    pg_pool: PgPool,
) -> Result<(), Box<dyn Error>> {
    let payload_bytes = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = serde_json::from_slice(&payload_bytes)?;
    let hendelse_type = json["hendelseType"].as_str().unwrap_or_default();
    let aarsak = json["metadata"]["aarsak"].as_str().unwrap_or_default();

    match hendelse_type == AVVIST_HENDELSE_TYPE && aarsak == AARSAK_UNDER_18 {
        true => {
            let hendelse: Hendelse = serde_json::from_value(json)?;
            let arbeidssoker_id = hendelse.id;
            let oppgave_id = hent_opg_id_for(arbeidssoker_id, pg_pool).await;
            log::info!("Prosesserer avvist hendelse for arbeidssøker");
        }
        false => { /* Gå videre */ }
    }
    Ok(())
}

async fn hent_opg_id_for(arbeidssoeker_id: i64, pg_pool: PgPool) {
    //hent oppgave id fra db hvis det finnes
}
