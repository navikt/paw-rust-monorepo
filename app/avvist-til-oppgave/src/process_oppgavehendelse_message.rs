use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::finn_oppgave_for_ekstern_id;
use crate::domain::oppgave_hendelse::OppgaveHendelseMelding;

pub async fn process_oppgavehendelse_message(
    kafka_message: &OwnedMessage,
    app_config: &ApplicationConfig,
    pg_pool: PgPool,
) -> anyhow::Result<()> {
    let hwm_version = *app_config.topic_oppgavehendelse_version;
    let mut tx = pg_pool.begin().await?;

    if update_hwm(
        &mut tx,
        hwm_version,
        kafka_message.topic(),
        kafka_message.partition(),
        kafka_message.offset(),
    )
        .await?
    {
        oppdater_oppgave_for_avvist_hendelse(kafka_message, app_config, &mut tx).await?;
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }

    Ok(())
}

async fn oppdater_oppgave_for_avvist_hendelse(
    kafka_message: &OwnedMessage,
    _app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let payload_bytes: Vec<u8> = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = serde_json::from_slice(&payload_bytes)?;
    let oppgave_hendelse: OppgaveHendelseMelding = serde_json::from_value(json.clone())?;
    let oppgave_id = oppgave_hendelse.oppgave.oppgave_id;

    let optional_oppgave = finn_oppgave_for_ekstern_id(oppgave_id, tx).await?;
    match optional_oppgave {
        None => {}
        Some(oppgave) => {}
    }

    Ok(())
}