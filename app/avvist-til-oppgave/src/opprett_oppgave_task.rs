use crate::client::oppgave_client::{OppgaveApiClient, OppgaveApiError};
use crate::client::opprett_oppgave_request::create_oppgave_request;
use crate::db::oppgave_functions::{
    hent_de_eldste_ubehandlede_oppgavene, insert_oppgave_hendelse_logg,
    oppdater_oppgave_med_ekstern_id, oppdater_oppgave_status,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
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
) -> Result<(), anyhow::Error> {
    let mut tx = db_pool.begin().await?;
    let mut oppgaver = hent_de_eldste_ubehandlede_oppgavene(batch_size, &mut tx).await?;
    oppgaver.shuffle(&mut rand::rng());
    tx.commit().await?;

    for oppgave in oppgaver {
        let mut tx = db_pool.begin().await?;

        if oppdater_oppgave_status(oppgave.id, Opprettet, &mut tx).await? {
            let opprett_oppgave_request = create_oppgave_request(oppgave.identitetsnummer.clone());
            let opprett_oppgave_result = oppgave_api_client
                .opprett_oppgave(&opprett_oppgave_request)
                .await;
            let oppgave_dto = match opprett_oppgave_result {
                Ok(oppgave_dto) => oppgave_dto,
                Err(error) => {
                    if rull_tilbake_til_ubehandlet_og_logg_feil(&oppgave, error, &mut tx).await? {
                        tx.commit().await?;
                    } else {
                        log::warn!("Klarte ikke å rulle tilbake oppgave med id {} til ubehandlet",oppgave.id);
                        tx.rollback().await?;
                    }
                    continue;
                }
            };

            if oppdater_oppgave_med_ekstern_id(oppgave.id, oppgave_dto.id, &mut tx).await? {
                tx.commit().await?;
            }
        }
    }
    Ok(())
}

async fn rull_tilbake_til_ubehandlet_og_logg_feil(
    oppgave: &Oppgave,
    error: OppgaveApiError,
    mut tx: &mut Transaction<'_, Postgres>,
) -> Result<bool> {
    let tilbakerullet_status = oppdater_oppgave_status(oppgave.id, Ubehandlet, &mut tx).await;
    if tilbakerullet_status.is_ok() {
        match error {
            OppgaveApiError::ApiError { status, message } => {
                let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
                    oppgave_id: oppgave.id,
                    status: EksternOppgaveOpprettelseFeilet.to_string(),
                    melding: format!(
                        "Feil ved opprettelse av oppgave i Oppgave API. Status: {}, message: {}",
                        status, message
                    ),
                    tidspunkt: Utc::now(),
                };
                insert_oppgave_hendelse_logg(&hendelse_logg_row, &mut tx).await?;
            }
            _ => return Err(error.into()),
        }
        Ok(true)
    } else {
        //TODO: Counter tilknyttet en alert eller noe
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{hent_oppgave, insert_oppgave};
    use crate::db::oppgave_row::InsertOppgaveRow;
    use async_trait::async_trait;
    use mockito::{Server, ServerGuard};
    use paw_test::setup_test_db::setup_test_db;
    use serde_json::json;
    use std::sync::Arc;
    use texas_client::{M2MTokenClient, TokenResponse};

    #[tokio::test]
    async fn test_prosesser_ubehandlede_oppgaver_happy_path() -> Result<()> {
        let ekstern_oppgave_id = 12345;
        let mock_server = start_oppgave_api_mock_server(ekstern_oppgave_id).await?;
        let oppgave_api_client = Arc::new(OppgaveApiClient::new(
            mock_server.url(),
            Arc::new(MockTokenClient),
        ));

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut tx = pg_pool.begin().await?;
        let ubehandlet_oppgave_row = InsertOppgaveRow {
            arbeidssoeker_id: 12345,
            status: Ubehandlet.to_string(),
            ..Default::default()
        };
        insert_oppgave(&ubehandlet_oppgave_row, &mut tx).await?;
        tx.commit().await?;

        let result = prosesser_ubehandlede_oppgaver(pg_pool.clone(), oppgave_api_client, BATCH_SIZE).await;
        assert!(result.is_ok());

        let mut tx = pg_pool.begin().await?;
        let oppdatert_oppgave = hent_oppgave(ubehandlet_oppgave_row.arbeidssoeker_id, &mut tx)
            .await?
            .unwrap();

        assert_eq!(
            oppdatert_oppgave.ekstern_oppgave_id.unwrap(),
            ekstern_oppgave_id
        );
        assert_eq!(oppdatert_oppgave.status, Opprettet);

        Ok(())
    }

    async fn start_oppgave_api_mock_server(ekstern_oppgave_id: i64) -> Result<ServerGuard> {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/api/v1/oppgaver")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": ekstern_oppgave_id,
                    "tildelt_enhetsnr": "4863",
                    "oppgavetype": "JFR",
                    "tema": "KON",
                    "prioritet": "NORM",
                    "status": "OPPRETTET",
                    "aktiv_dato": "2026-02-16",
                    "versjon": 1,
                })
                .to_string(),
            )
            .create_async()
            .await;
        Ok(server)
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
