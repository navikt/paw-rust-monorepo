use crate::db::oppgave_functions::{
    bytt_oppgave_status, finn_oppgave_for_ekstern_id, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave_hendelse::{OppgaveHendelseMelding, OppgaveHendelsetype};
use crate::domain::oppgave_status::OppgaveStatus;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use OppgaveHendelsetype::{OppgaveFeilregistrert, OppgaveFerdigstilt};
use OppgaveStatus::{Ferdigbehandlet, Opprettet};

pub async fn oppdater_ferdigstilte_oppgaver(
    kafka_message: &OwnedMessage,
    opprett_oppgaver_fra_tidspunkt: DateTime<Utc>,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let payload_bytes: Vec<u8> = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = match serde_json::from_slice(&payload_bytes) {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };

    let hendelsestype = json["hendelse"]["hendelsestype"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    if hendelsestype != OppgaveFerdigstilt.to_string()
        && hendelsestype != OppgaveFeilregistrert.to_string()
    {
        return Ok(());
    }

    let oppgave_hendelse: OppgaveHendelseMelding =
        serde_json::from_value(json).context("Kunne ikke deserialisere oppgavehendelse")?;

    // Tidspunktet fra oppgave-appen er i Oslo-tid (TZ="Europe/Oslo" i Dockerfile)
    let hendelse_tidspunkt = oslo_tid_til_utc(oppgave_hendelse.hendelse.tidspunkt);
    if hendelse_tidspunkt < opprett_oppgaver_fra_tidspunkt {
        return Ok(());
    }

    let ekstern_oppgave_id = oppgave_hendelse.oppgave.oppgave_id;
    let oppgave = match finn_oppgave_for_ekstern_id(ekstern_oppgave_id, tx).await? {
        None => return Ok(()),
        Some(oppgave) => oppgave,
    };

    let logg_status = match oppgave_hendelse.hendelse.hendelsestype {
        OppgaveFerdigstilt => HendelseLoggStatus::EksternOppgaveFerdigstilt,
        OppgaveFeilregistrert => HendelseLoggStatus::EksternOppgaveFeilregistrert,
        OppgaveHendelsetype::OppgaveOpprettet | OppgaveHendelsetype::OppgaveEndret => {
            unreachable!("Filtrert bort av hendelsestype-sjekken over")
        }
    };

    if bytt_oppgave_status(oppgave.id, Opprettet, Ferdigbehandlet, tx).await? {
        let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
            oppgave_id: oppgave.id,
            status: logg_status.to_string(),
            melding: format!(
                "Ekstern oppgave {} ble {}",
                ekstern_oppgave_id,
                hendelsestype.to_lowercase()
            ),
            tidspunkt: Utc::now(),
        };
        insert_oppgave_hendelse_logg(&hendelse_logg_row, tx).await?;
        tracing::info!(
            "Oppgave {} oppdatert til Ferdigbehandlet etter melding om ekstern {}",
            oppgave.id,
            hendelsestype.to_lowercase()
        );
    }

    Ok(())
}

fn oslo_tid_til_utc(tidspunkt: chrono::NaiveDateTime) -> DateTime<Utc> {
    tidspunkt
        .and_local_timezone(chrono_tz::Europe::Oslo)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| {
            tracing::warn!(
                "Kunne ikke konvertere tidspunkt {:?} til Oslo-tid, faller tilbake til UTC",
                tidspunkt
            );
            tidspunkt.and_utc()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{
        hent_nyeste_oppgave, insert_oppgave, oppdater_oppgave_med_ekstern_id,
    };
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use HendelseLoggStatus::{EksternOppgaveFeilregistrert, EksternOppgaveFerdigstilt};

    const FRA_TIDSPUNKT: DateTime<Utc> = DateTime::UNIX_EPOCH;
    const EKSTERN_OPPGAVE_ID_FERDIGSTILT: i64 = 55555;
    const EKSTERN_OPPGAVE_ID_FEILREGISTRERT: i64 = 66666;

    #[tokio::test]
    async fn test_irrelevante_meldinger_ignoreres() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let ugyldig_message = lag_kafka_melding(b"dette er ikke json".to_vec());
        let mut tx = pg_pool.begin().await?;
        assert!(
            oppdater_ferdigstilte_oppgaver(&ugyldig_message, FRA_TIDSPUNKT, &mut tx)
                .await
                .is_ok()
        );
        tx.commit().await?;

        let irrelevant_message = lag_kafka_melding(OPPGAVE_OPPRETTET_JSON.as_bytes().to_vec());
        let mut tx = pg_pool.begin().await?;
        assert!(
            oppdater_ferdigstilte_oppgaver(&irrelevant_message, FRA_TIDSPUNKT, &mut tx)
                .await
                .is_ok()
        );
        tx.commit().await?;

        let ukjent_message = lag_kafka_melding(OPPGAVE_FERDIGSTILT_JSON.as_bytes().to_vec());
        let mut tx = pg_pool.begin().await?;
        assert!(
            oppdater_ferdigstilte_oppgaver(&ukjent_message, FRA_TIDSPUNKT, &mut tx)
                .await
                .is_ok()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_ferdigstilt_og_feilregistrert_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // --- Ferdigstilt ---
        let arbeidssoeker_id_1 = 12345;
        let mut tx = pg_pool.begin().await?;
        let oppgave_id = insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id: arbeidssoeker_id_1,
                status: Opprettet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        oppdater_oppgave_med_ekstern_id(oppgave_id, EKSTERN_OPPGAVE_ID_FERDIGSTILT, &mut tx)
            .await?;
        tx.commit().await?;

        let message = lag_kafka_melding(OPPGAVE_FERDIGSTILT_JSON.as_bytes().to_vec());
        let mut tx = pg_pool.begin().await?;
        oppdater_ferdigstilte_oppgaver(&message, FRA_TIDSPUNKT, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id_1, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ferdigbehandlet);
        assert!(
            oppgave
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveFerdigstilt),
            "Forventet EksternOppgaveFerdigstilt i hendelseloggen, fant: {:?}",
            oppgave.hendelse_logg
        );
        tx.commit().await?;

        // Duplikat ferdigstilling skal ikke legge til ny hendelseslogg
        let message = lag_kafka_melding(OPPGAVE_FERDIGSTILT_JSON.as_bytes().to_vec());
        let mut tx = pg_pool.begin().await?;
        oppdater_ferdigstilte_oppgaver(&message, FRA_TIDSPUNKT, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id_1, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ferdigbehandlet);
        assert_eq!(
            oppgave.hendelse_logg.len(),
            1,
            "Forventet kun 1 hendelseslogg-entry, fant: {:?}",
            oppgave.hendelse_logg
        );
        tx.commit().await?;

        // --- Feilregistrert ---
        let arbeidssoeker_id_2 = 67890;
        let mut tx = pg_pool.begin().await?;
        let oppgave_id = insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id: arbeidssoeker_id_2,
                status: Opprettet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        oppdater_oppgave_med_ekstern_id(oppgave_id, EKSTERN_OPPGAVE_ID_FEILREGISTRERT, &mut tx)
            .await?;
        tx.commit().await?;

        let message = lag_kafka_melding(OPPGAVE_FEILREGISTRERT_JSON.as_bytes().to_vec());
        let mut tx = pg_pool.begin().await?;
        oppdater_ferdigstilte_oppgaver(&message, FRA_TIDSPUNKT, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id_2, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ferdigbehandlet);
        assert!(
            oppgave
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveFeilregistrert),
            "Forventet EksternOppgaveFeilregistrert i hendelseloggen, fant: {:?}",
            oppgave.hendelse_logg
        );

        Ok(())
    }

    fn lag_kafka_melding(payload: Vec<u8>) -> OwnedMessage {
        OwnedMessage::new(
            Some(payload),
            None,
            "test-topic".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        )
    }

    //language=JSON
    const OPPGAVE_FERDIGSTILT_JSON: &str = r#"{
        "hendelse": {
            "hendelsestype": "OPPGAVE_FERDIGSTILT",
            "tidspunkt": [2023, 2, 23, 8, 58, 23, 832000000]
        },
        "utfortAv": {
            "navIdent": "Z991459",
            "enhetsnr": "2990"
        },
        "oppgave": {
            "oppgaveId": 55555,
            "versjon": 2,
            "tilordning": null,
            "kategorisering": null,
            "behandlingsperiode": null,
            "bruker": null
        }
    }"#;

    //language=JSON
    const OPPGAVE_FEILREGISTRERT_JSON: &str = r#"{
        "hendelse": {
            "hendelsestype": "OPPGAVE_FEILREGISTRERT",
            "tidspunkt": [2023, 2, 23, 8, 58, 23, 832000000]
        },
        "utfortAv": {
            "navIdent": "Z991459",
            "enhetsnr": "2990"
        },
        "oppgave": {
            "oppgaveId": 66666,
            "versjon": 2,
            "tilordning": null,
            "kategorisering": null,
            "behandlingsperiode": null,
            "bruker": null
        }
    }"#;

    //language=JSON
    const OPPGAVE_OPPRETTET_JSON: &str = r#"{
        "hendelse": {
            "hendelsestype": "OPPGAVE_OPPRETTET",
            "tidspunkt": [2023, 2, 23, 8, 58, 23, 832000000]
        },
        "utfortAv": null,
        "oppgave": {
            "oppgaveId": 99999,
            "versjon": 1,
            "tilordning": null,
            "kategorisering": null,
            "behandlingsperiode": null,
            "bruker": null
        }
    }"#;
}
