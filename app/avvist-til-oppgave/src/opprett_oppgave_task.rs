use crate::client::oppgave_client::{OppgaveApiClient, OppgaveApiError};
use crate::client::opprett_oppgave_request::create_oppgave_request;
use crate::db::oppgave_functions::{
    hent_de_eldste_ubehandlede_oppgavene, insert_oppgave_hendelse_logg,
    oppdater_oppgave_med_ekstern_id, bytt_oppgave_status,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::hendelse_logg_status::HendelseLoggStatus::EksternOppgaveOpprettet;
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus::{Opprettet, Ubehandlet};
use HendelseLoggStatus::EksternOppgaveOpprettelseFeilet;
use anyhow::Result;
use chrono::Utc;
use rand::prelude::*;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use tokio::time::{Duration, interval};

const BATCH_SIZE: i64 = 10;

pub async fn start_processing_loop(
    db_pool: PgPool,
    oppgave_api_client: Arc<OppgaveApiClient>,
) -> Result<(), anyhow::Error> {
    let mut interval = interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        /*
        if let Err(e) = prosesser_ubehandlede_oppgaver(db_pool.clone(), oppgave_api_client.clone(), BATCH_SIZE).await {
            log::error!("Feil i prosesseringsloop: {}", e);
        }
        */
    }
}

async fn prosesser_ubehandlede_oppgaver(
    db_pool: PgPool,
    oppgave_api_client: Arc<OppgaveApiClient>,
    batch_size: i64,
) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    let mut oppgaver = hent_de_eldste_ubehandlede_oppgavene(batch_size, &mut tx).await?;
    oppgaver.shuffle(&mut rand::rng());
    tx.commit().await?;

    for oppgave in oppgaver {
        let mut tx = db_pool.begin().await?;

        if !bytt_oppgave_status(oppgave.id, Ubehandlet, Opprettet, &mut tx).await? {
            tx.rollback().await?;
            continue;
        }

        let opprett_oppgave_request = create_oppgave_request(oppgave.identitetsnummer.clone());
        let opprett_oppgave_result = oppgave_api_client
            .opprett_oppgave(&opprett_oppgave_request)
            .await;

        match opprett_oppgave_result {
            Ok(oppgave_dto) => {
                let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
                    oppgave_id: oppgave.id,
                    status: EksternOppgaveOpprettet.to_string(),
                    melding: "Ekstern oppgave_id opprettet".to_string(),
                    tidspunkt: Utc::now(),
                };
                insert_oppgave_hendelse_logg(&hendelse_logg_row, &mut tx).await?;
                if !oppdater_oppgave_med_ekstern_id(oppgave.id, oppgave_dto.id, &mut tx).await? {
                    log::warn!("Feilet update av ekstern_oppgave_id for oppgave med id {}", oppgave.id);
                    tx.rollback().await?;
                    continue;
                }
                tx.commit().await?;
            }
            Err(error) => {
                match set_ubehandlet_og_logg_feil(&oppgave, error, &mut tx).await {
                    Ok(true) => tx.commit().await?,
                    Ok(false) => {
                        log::warn!("Feilet update av oppgave med id {} til ubehandlet", oppgave.id);
                        tx.rollback().await?;
                    }
                    Err(e) => {
                        log::error!("Feil ved feilhåndtering for oppgave {}: {}", oppgave.id, e);
                        tx.rollback().await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn set_ubehandlet_og_logg_feil(
    oppgave: &Oppgave,
    error: OppgaveApiError,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<bool> {
    if !bytt_oppgave_status(oppgave.id, Opprettet, Ubehandlet, tx).await? {
        return Ok(false);
    }

    let error_melding = match error {
        OppgaveApiError::ApiError { status, message } => {
            format!(
                "Feil ved opprettelse av oppgave i Oppgave API. Status: {}, message: {}",
                status, message
            )
        }
        OppgaveApiError::ReqwestError(e) => {
            format!("HTTP-feil ved kall til Oppgave API: {}", e)
        }
        OppgaveApiError::TokenError(e) => {
            format!("Token-feil ved kall til Oppgave API: {}", e)
        }
    };

    let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
        oppgave_id: oppgave.id,
        status: EksternOppgaveOpprettelseFeilet.to_string(),
        melding: error_melding,
        tidspunkt: Utc::now(),
    };
    let rows_affected = insert_oppgave_hendelse_logg(&hendelse_logg_row, tx).await?;
    Ok(rows_affected == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::oppgave_client::OPPGAVER_PATH;
    use crate::db::oppgave_functions::{hent_oppgave, insert_oppgave};
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus::EksternOppgaveOpprettet;
    use async_trait::async_trait;
    use mockito::{Matcher, Server};
    use paw_test::setup_test_db::setup_test_db;
    use serde_json::json;
    use std::sync::Arc;
    use texas_client::{M2MTokenClient, TokenResponse};

    #[tokio::test]
    async fn prosesser_tre_oppgaver_to_ok_en_feil() -> Result<()> {
        let identitetsnummer_1 = "12345678901";
        let identitetsnummer_2 = "12345678902";
        let identitetsnummer_3 = "12345678903";

        let mut server = Server::new_async().await;

        server
            .mock("POST", OPPGAVER_PATH)
            .match_body(Matcher::Regex(format!(r#""personident":"{}"#, identitetsnummer_1)))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(json!({"id": 100, "tildeltEnhetsnr": "4863", "oppgavetype": "JFR", "tema": "KON", "prioritet": "NORM", "status": "OPPRETTET", "aktivDato": "2026-02-16", "versjon": 1}).to_string())
            .create_async()
            .await;

        server
            .mock("POST", OPPGAVER_PATH)
            .match_body(Matcher::Regex(format!(r#""personident":"{}"#, identitetsnummer_2)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(json!({"message": "Ugyldig identitetsnummer", "errorCode": "VALIDATION_ERROR"}).to_string())
            .create_async()
            .await;

        server
            .mock("POST", OPPGAVER_PATH)
            .match_body(Matcher::Regex(format!(r#""personident":"{}"#, identitetsnummer_3)))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(json!({"id": 200, "tildeltEnhetsnr": "4863", "oppgavetype": "JFR", "tema": "KON", "prioritet": "NORM", "status": "OPPRETTET", "aktivDato": "2026-02-16", "versjon": 1}).to_string())
            .create_async()
            .await;

        let oppgave_api_client = Arc::new(OppgaveApiClient::new(
            server.url(),
            Arc::new(MockTokenClient),
        ));

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Sett inn 3 oppgaver
        let mut tx = pg_pool.begin().await?;

        let arbeidssoeker_id_1 = 12345;
        insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id: arbeidssoeker_id_1,
                identitetsnummer: identitetsnummer_1.to_string(),
                status: Ubehandlet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;

        let arbeidssoeker_id_2 = 12346;
        insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id: arbeidssoeker_id_2,
                identitetsnummer: identitetsnummer_2.to_string(),
                status: Ubehandlet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;

        let arbeidssoeker_id_3 = 12347;
        insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id: arbeidssoeker_id_3,
                identitetsnummer: identitetsnummer_3.to_string(),
                status: Ubehandlet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        let result = prosesser_ubehandlede_oppgaver(pg_pool.clone(), oppgave_api_client, 3).await;
        assert!(result.is_ok(), "Funksjonen skulle returnere Ok(())");

        let mut tx = pg_pool.begin().await?;

        // Oppgave 1: vellykket
        let oppgave_1 = hent_oppgave(arbeidssoeker_id_1, &mut tx).await?.unwrap();
        assert_eq!(oppgave_1.status, Opprettet);
        assert_eq!(oppgave_1.ekstern_oppgave_id, Some(100));
        assert!(
            oppgave_1
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveOpprettet)
        );

        // Oppgave 2: feilet — tilbake til Ubehandlet med feil-logg
        let oppgave_2 = hent_oppgave(arbeidssoeker_id_2, &mut tx).await?.unwrap();
        assert_eq!(oppgave_2.status, Ubehandlet);
        assert!(oppgave_2.ekstern_oppgave_id.is_none());
        assert!(
            oppgave_2
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveOpprettelseFeilet)
        );

        // Oppgave 3: vellykket
        let oppgave_3 = hent_oppgave(arbeidssoeker_id_3, &mut tx).await?.unwrap();
        assert_eq!(oppgave_3.status, Opprettet);
        assert_eq!(oppgave_3.ekstern_oppgave_id, Some(200));
        assert!(
            oppgave_3
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveOpprettet)
        );

        Ok(())
    }

    #[tokio::test]
    async fn prosesser_tom_batch() -> Result<()> {
        let server = Server::new_async().await;
        let oppgave_api_client = Arc::new(OppgaveApiClient::new(
            server.url(),
            Arc::new(MockTokenClient),
        ));
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let result = prosesser_ubehandlede_oppgaver(pg_pool.clone(), oppgave_api_client, 10).await;
        assert!(result.is_ok());
        Ok(())
    }


    struct MockTokenClient;
    #[async_trait]
    impl M2MTokenClient for MockTokenClient {
        async fn get_token(&self, _target: String) -> Result<TokenResponse> {
            Ok(TokenResponse {
                access_token: "dummy-token".to_string(),
                expires_in: 3600,
                token_type: "Bearer".to_string(),
            })
        }
    }
}
