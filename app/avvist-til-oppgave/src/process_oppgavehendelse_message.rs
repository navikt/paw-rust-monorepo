use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::{
    bytt_oppgave_status, finn_oppgave_for_ekstern_id, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave_hendelse::{OppgaveHendelseMelding, OppgaveHendelsetype};
use crate::domain::oppgave_status::OppgaveStatus;
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use OppgaveHendelsetype::OppgaveFerdigstilt;
use OppgaveStatus::Ferdigbehandlet;

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
        oppdater_ferdigstilte_oppgaver(kafka_message, app_config, &mut tx).await?;
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }

    Ok(())
}

async fn oppdater_ferdigstilte_oppgaver(
    kafka_message: &OwnedMessage,
    _app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let payload_bytes: Vec<u8> = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = serde_json::from_slice(&payload_bytes)?;
    let oppgave_hendelse: OppgaveHendelseMelding = serde_json::from_value(json.clone())?;

    if oppgave_hendelse.hendelse.hendelsestype != OppgaveFerdigstilt {
        return Ok(());
    }

    log::info!("Ferdigstilt oppgavehendelse mottatt: {}", json);

    let ekstern_oppgave_id = oppgave_hendelse.oppgave.oppgave_id;
    let oppgave = match finn_oppgave_for_ekstern_id(ekstern_oppgave_id, tx).await? {
        None => return Ok(()),
        Some(oppgave) => oppgave,
    };

    if oppgave.status == Ferdigbehandlet {
        tracing::info!(
            "Oppgave {} er allerede ferdigbehandlet, ignorerer",
            oppgave.id
        );
        return Ok(());
    }

    if bytt_oppgave_status(
        oppgave.id,
        oppgave.status.clone(), //vil vel alltid være Opprettet?
        Ferdigbehandlet,
        tx,
    )
    .await?
    {
        let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
            oppgave_id: oppgave.id,
            status: HendelseLoggStatus::EksternOppgaveFerdigstilt.to_string(),
            melding: format!("Ekstern oppgave {} ble ferdigstilt", ekstern_oppgave_id),
            tidspunkt: chrono::Utc::now(),
        };
        insert_oppgave_hendelse_logg(&hendelse_logg_row, tx).await?;
        tracing::info!(
            "Oppgave {} oppdatert til Ferdigbehandlet etter melding om ekstern ferdigstilling",
            oppgave.id
        );
    } else {
        tracing::warn!(
            "Kunne ikke oppdatere oppgave {} til Ferdigbehandlet",
            oppgave.id
        )
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_application_config;
    use crate::db::oppgave_functions::{
        hent_oppgave, insert_oppgave, oppdater_oppgave_med_ekstern_id,
    };
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use crate::domain::oppgave_status::OppgaveStatus;
    use anyhow::Result;
    use chrono::Utc;
    use paw_rdkafka_hwm::hwm_functions::insert_hwm;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use serde_json::json;

    #[tokio::test]
    async fn test_ferdigstilt_oppgave_oppdaterer_status() -> Result<()> {
        let app_config = read_application_config()?;
        let hwm_version = *app_config.topic_oppgavehendelse_version;
        let topic = app_config.topic_oppgavehendelse.to_string();

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Setup HWM
        let mut tx = pg_pool.begin().await?;
        insert_hwm(&mut tx, hwm_version, topic.as_str(), 0, 0).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = 12345;
        let oppgave_id = insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id,
                status: OppgaveStatus::Opprettet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        oppdater_oppgave_med_ekstern_id(oppgave_id, EKSTERN_OPPGAVE_ID, &mut tx).await?;
        tx.commit().await?;

        let payload = oppgave_ferdigstilt_json(EKSTERN_OPPGAVE_ID);
        let message = OwnedMessage::new(
            Some(payload.into_bytes()),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        process_oppgavehendelse_message(&message, &app_config, pg_pool.clone()).await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_oppgave(arbeidssoeker_id, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, Ferdigbehandlet);
        assert!(
            oppgave
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == HendelseLoggStatus::EksternOppgaveFerdigstilt),
            "Forventet EksternOppgaveFerdigstilt i hendelseloggen, fant: {:?}",
            oppgave.hendelse_logg
        );

        Ok(())
    }

    const EKSTERN_OPPGAVE_ID: i64 = 55555;

    fn oppgave_ferdigstilt_json(oppgave_id: i64) -> String {
        json!({
            "hendelse": {
                "hendelsestype": "OPPGAVE_FERDIGSTILT",
                "tidspunkt": [2023, 2, 23, 8, 58, 23, 832000000]
            },
            "utfortAv": {
                "navIdent": "Z991459",
                "enhetsnr": "2990"
            },
            "oppgave": {
                "oppgaveId": oppgave_id,
                "versjon": 2,
                "tilordning": null,
                "kategorisering": null,
                "behandlingsperiode": null,
                "bruker": null
            }
        })
        .to_string()
    }
}
