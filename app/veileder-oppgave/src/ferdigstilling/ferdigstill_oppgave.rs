use crate::db::oppgave_functions::{
    bytt_oppgave_status, finn_oppgave_for_ekstern_id, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave_status::OppgaveStatus;
use HendelseLoggStatus::{EksternOppgaveFeilregistrert, EksternOppgaveFerdigstilt};
use OppgaveStatus::{Ferdigbehandlet, Opprettet};
use chrono::Utc;
use sqlx::{Postgres, Transaction};
use crate::domain::ekstern_oppgave_id::EksternOppgaveId;
use crate::ferdigstilling::oppgave_hendelse::{OppgaveHendelseMelding, OppgaveHendelsetype};

pub async fn ferdigstill_oppgave(
    kafka_message_payload: &[u8],
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let melding: OppgaveHendelseMelding = match serde_json::from_slice(kafka_message_payload) {
        Ok(melding) => melding,
        Err(_) => {
            tracing::warn!("Feilet deserialisering av oppgavehendelse, hopper over");
            return Ok(());
        }
    };

    let logg_status = match melding.hendelse.hendelsestype {
        OppgaveHendelsetype::OppgaveFerdigstilt => EksternOppgaveFerdigstilt,
        OppgaveHendelsetype::OppgaveFeilregistrert => EksternOppgaveFeilregistrert,
        OppgaveHendelsetype::OppgaveOpprettet => return Ok(()),
        OppgaveHendelsetype::OppgaveEndret => return Ok(()),
    };

    let ekstern_oppgave_id = EksternOppgaveId::from(melding.oppgave.oppgave_id);
    let oppgave = match finn_oppgave_for_ekstern_id(ekstern_oppgave_id, tx).await? {
        None => return Ok(()),
        Some(oppgave) => oppgave,
    };

    if bytt_oppgave_status(oppgave.id, Opprettet, Ferdigbehandlet, tx).await? {
        let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
            oppgave_id: oppgave.id,
            status: logg_status.to_string(),
            melding: format!(
                "Ekstern oppgave {} ble {}",
                ekstern_oppgave_id,
                melding.hendelse.hendelsestype.to_string().to_lowercase()
            ),
            tidspunkt: Utc::now(),
        };
        insert_oppgave_hendelse_logg(&hendelse_logg_row, tx).await?;
        tracing::info!(
            "Oppgave {} oppdatert til Ferdigbehandlet etter melding om ekstern {}",
            oppgave.id,
            melding.hendelse.hendelsestype.to_string().to_lowercase()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{
        hent_nyeste_oppgave, insert_oppgave, oppdater_oppgave_med_ekstern_id,
    };
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use crate::domain::oppgave_type::OppgaveType;

    use HendelseLoggStatus::{EksternOppgaveFeilregistrert, EksternOppgaveFerdigstilt};
    use anyhow::Result;
    use paw_test::setup_test_db::setup_test_db;
    use crate::domain::ekstern_oppgave_id::EksternOppgaveId;

    const EKSTERN_OPPGAVE_ID_FERDIGSTILT: i64 = 55555;
    const EKSTERN_OPPGAVE_ID_FEILREGISTRERT: i64 = 66666;

    #[tokio::test]
    async fn test_irrelevante_meldinger_ignoreres() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let ugyldig_message = "dette er ikke json".as_bytes();
        let mut tx = pg_pool.begin().await?;
        assert!(ferdigstill_oppgave(ugyldig_message, &mut tx).await.is_ok());
        tx.commit().await?;

        let irrelevant_message = OPPGAVE_OPPRETTET_JSON.as_bytes();
        let mut tx = pg_pool.begin().await?;
        assert!(
            ferdigstill_oppgave(irrelevant_message, &mut tx)
                .await
                .is_ok()
        );
        tx.commit().await?;

        let ukjent_message = OPPGAVE_FERDIGSTILT_JSON.as_bytes();
        let mut tx = pg_pool.begin().await?;
        assert!(ferdigstill_oppgave(ukjent_message, &mut tx).await.is_ok());

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
        oppdater_oppgave_med_ekstern_id(oppgave_id, EksternOppgaveId::from(EKSTERN_OPPGAVE_ID_FERDIGSTILT), &mut tx)
            .await?;
        tx.commit().await?;

        let message = OPPGAVE_FERDIGSTILT_JSON.as_bytes();
        let mut tx = pg_pool.begin().await?;
        ferdigstill_oppgave(message, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id_1, OppgaveType::AvvistUnder18, &mut tx)
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
        let message = OPPGAVE_FERDIGSTILT_JSON.as_bytes();
        let mut tx = pg_pool.begin().await?;
        ferdigstill_oppgave(message, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id_1, OppgaveType::AvvistUnder18, &mut tx)
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
        oppdater_oppgave_med_ekstern_id(oppgave_id, EksternOppgaveId::from(EKSTERN_OPPGAVE_ID_FEILREGISTRERT), &mut tx)
            .await?;
        tx.commit().await?;

        let message = OPPGAVE_FEILREGISTRERT_JSON.as_bytes();
        let mut tx = pg_pool.begin().await?;
        ferdigstill_oppgave(message, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id_2, OppgaveType::AvvistUnder18, &mut tx)
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
