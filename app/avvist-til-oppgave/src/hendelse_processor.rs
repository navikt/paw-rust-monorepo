use crate::avvist_hendelse::AvvistHendelse;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use rdkafka::message::Message;
use rdkafka::message::OwnedMessage;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
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
        let tx = pg_pool.begin().await?;
        process_hendelse(&kafka_message, tx).await?;
    }
}

const HWM_VERSION: i16 = 1;

pub async fn process_hendelse(
    kafka_message: &OwnedMessage,
    mut tx: Transaction<'_, Postgres>,
) -> Result<(), Box<dyn Error>> {
    match update_hwm(
        &mut tx,
        HWM_VERSION,
        kafka_message.topic(),
        kafka_message.partition(),
        kafka_message.offset(),
    )
    .await?
    {
        true => {
            hugga_bugga(kafka_message, &mut tx).await?;
            tx.commit().await?;
        }
        false => {
            tx.rollback().await?;
        }
    }
    Ok(())
}

async fn hugga_bugga(
    kafka_message: &OwnedMessage,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), Box<dyn Error>> {
    let payload_bytes = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = serde_json::from_slice(&payload_bytes)?;
    let hendelse_type = json["hendelseType"].as_str().unwrap_or_default();
    let aarsak = json["metadata"]["aarsak"].as_str().unwrap_or_default();

    match hendelse_type == AVVIST_HENDELSE_TYPE && aarsak == AARSAK_UNDER_18 {
        true => {
            let hendelse: AvvistHendelse = serde_json::from_value(json)?;
            //insert_ubehandlet_avvist_melding(&hendelse.into(), tx).await?;
            log::info!("Prosesserer avvist hendelse for arbeidssøker");
            tracing::info!("Prosesserer avvist hendelse for arbeidssøker");
        }
        false => { /* Gå videre */ }
    }
    Ok(())
}
