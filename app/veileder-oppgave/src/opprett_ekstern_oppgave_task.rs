use crate::client::oppgave_client::{OppgaveApiClient, OppgaveApiError};
use crate::client::opprett_oppgave_request::create_oppgave_request;
use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::{
    bytt_oppgave_status, hent_de_eldste_ubehandlede_oppgavene, insert_oppgave_hendelse_logg,
    oppdater_oppgave_med_ekstern_id,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::domain::hendelse_logg_status::HendelseLoggStatus::{
    EksternOppgaveOpprettelseFeilet, EksternOppgaveOpprettet,
};
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus::{Opprettet, Ubehandlet};
use crate::metrics::ekstern_oppgave_opprettelse_feil::inkrement_ekstern_oppgave_opprettelse_feil;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rand::prelude::*;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::interval;

pub fn spawn_ekstern_oppgave_task(
    db_pool: PgPool,
    oppgave_api_client: Arc<OppgaveApiClient>,
    app_config: ApplicationConfig,
) -> JoinHandle<Result<()>> {
    tokio::spawn(kjør_processing_loop(
        db_pool,
        oppgave_api_client,
        app_config,
    ))
}

async fn kjør_processing_loop(
    db_pool: PgPool,
    oppgave_api_client: Arc<OppgaveApiClient>,
    app_config: ApplicationConfig,
) -> Result<()> {
    let opprett_oppgaver_task_interval_minutes = *app_config.opprett_oppgaver_task_interval_minutes;
    let opprett_oppgaver_task_batch_size = *app_config.opprett_oppgaver_task_batch_size;
    let opprett_avvist_under_18_oppgaver_fra_tidspunkt =
        *app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt;
    let task_interval = Duration::from_mins(opprett_oppgaver_task_interval_minutes);
    let mut interval = interval(task_interval);
    loop {
        interval.tick().await;
        if let Err(e) = prosesser_ubehandlede_oppgaver(
            opprett_avvist_under_18_oppgaver_fra_tidspunkt,
            opprett_oppgaver_task_batch_size,
            oppgave_api_client.clone(),
            db_pool.clone(),
        )
        .await
        {
            tracing::error!("Feil i prosesseringsloop: {}", e);
        }
    }
}

pub async fn prosesser_ubehandlede_oppgaver(
    fra_tidspunkt: DateTime<Utc>,
    batch_size: i64,
    oppgave_api_client: Arc<OppgaveApiClient>,
    db_pool: PgPool,
) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    let mut oppgaver =
        hent_de_eldste_ubehandlede_oppgavene(batch_size, fra_tidspunkt, &mut tx).await?;
    oppgaver.shuffle(&mut rand::rng());
    tx.commit().await?;

    for oppgave in oppgaver {
        if let Err(e) = prosesser_oppgave(&db_pool, &oppgave_api_client, &oppgave).await {
            tracing::error!("Feil ved prosessering av oppgave {}: {}", oppgave.id, e);
        }
    }
    Ok(())
}

async fn prosesser_oppgave(
    db_pool: &PgPool,
    oppgave_client: &OppgaveApiClient,
    oppgave: &Oppgave,
) -> Result<()> {
    let mut tx = db_pool.begin().await?;

    // CAS: prøv å ta eierskap over oppgaven
    if !bytt_oppgave_status(oppgave.id, Ubehandlet, Opprettet, &mut tx).await? {
        tx.rollback().await?;
        tracing::info!("Oppgave {} tatt av en annen tråd, ignorerer", oppgave.id);
        return Ok(());
    }

    warning_ved_gjentatte_feil(oppgave);

    let opprett_oppgave_request =
        create_oppgave_request(oppgave.identitetsnummer.clone(), &oppgave.type_);
    let response = oppgave_client
        .opprett_oppgave(&opprett_oppgave_request)
        .await;

    match response {
        Ok(oppgave_dto) => {
            insert_oppgave_hendelse_logg(
                &InsertOppgaveHendelseLoggRow {
                    oppgave_id: oppgave.id,
                    status: EksternOppgaveOpprettet.to_string(),
                    melding: "Ekstern oppgave_id opprettet".to_string(),
                    tidspunkt: Utc::now(),
                },
                &mut tx,
            )
            .await?;
            oppdater_oppgave_med_ekstern_id(oppgave.id, oppgave_dto.id, &mut tx).await?;
            tracing::info!("Oppgave {} opprettet i Oppgave API", oppgave.id);
            tx.commit().await?;
        }
        Err(error) => {
            inkrement_ekstern_oppgave_opprettelse_feil(&error);
            if !bytt_oppgave_status(oppgave.id, Opprettet, Ubehandlet, &mut tx).await? {
                tracing::error!(
                    oppgave_id = %oppgave.id,
                    "Oppgave sitter fast i status Opprettet uten ekstern_id — manuell gjennomgang nødvendig"
                );
                return Err(anyhow::anyhow!(
                    "Kunne ikke sette oppgave {} tilbake til Ubehandlet",
                    oppgave.id
                ));
            }

            let error_melding = match &error {
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
            tracing::error!(
                "Feil ved opprettelse av oppgave {} i Oppgave API: {}",
                oppgave.id,
                error_melding
            );

            insert_oppgave_hendelse_logg(
                &InsertOppgaveHendelseLoggRow {
                    oppgave_id: oppgave.id,
                    status: EksternOppgaveOpprettelseFeilet.to_string(),
                    melding: error_melding,
                    tidspunkt: Utc::now(),
                },
                &mut tx,
            )
            .await?;
            tx.commit().await?;
        }
    }

    Ok(())
}

fn warning_ved_gjentatte_feil(oppgave: &Oppgave) {
    let antall_tidligere_feil = oppgave
        .hendelse_logg
        .iter()
        .filter(|innslag| innslag.status == EksternOppgaveOpprettelseFeilet)
        .count();

    if antall_tidligere_feil >= 5 {
        tracing::warn!(
            oppgave_id = %oppgave.id,
            antall_feil = antall_tidligere_feil,
            "Oppgave har feilet gjentatte ganger mot Oppgave API — mulig kork"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::oppgave_client::OPPGAVER_PATH;
    use crate::config::OppgaveClientConfig;
    use crate::db::oppgave_functions::{
        hent_de_eldste_ubehandlede_oppgavene, hent_nyeste_oppgave, insert_oppgave,
    };
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus::EksternOppgaveOpprettet;
    use crate::domain::oppgave_type::OppgaveType;
    use mockito::{Matcher, Server};
    use paw_test::setup_test_db::setup_test_db;
    use paw_test::stub_token_client::StubTokenClient;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn en_feilet_oppgave_stopper_ikke_batchen() -> Result<()> {
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
            .match_body(Matcher::Regex(format!(
                r#""personident":"{}"#,
                identitetsnummer_2
            )))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                json!({"message": "Ugyldig identitetsnummer", "errorCode": "VALIDATION_ERROR"})
                    .to_string(),
            )
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

        let base_url = server.url();
        let oppgave_api_client = Arc::new(OppgaveApiClient::new(
            OppgaveClientConfig {
                base_url: base_url.into(),
                scope: "test-scope".to_string().into(),
            },
            Arc::new(StubTokenClient),
        ));

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Tom batch: ingen oppgaver, ingen HTTP-kall forventet
        prosesser_ubehandlede_oppgaver(
            DateTime::UNIX_EPOCH,
            3,
            Arc::clone(&oppgave_api_client),
            pg_pool.clone(),
        )
        .await?;

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

        let fra_dato = DateTime::UNIX_EPOCH;
        let result =
            prosesser_ubehandlede_oppgaver(fra_dato, 3, oppgave_api_client, pg_pool.clone()).await;
        assert!(result.is_ok(), "Funksjonen skulle returnere Ok(())");

        let mut tx = pg_pool.begin().await?;

        // Oppgave 1: vellykket
        let oppgave_1 =
            hent_nyeste_oppgave(arbeidssoeker_id_1, OppgaveType::AvvistUnder18, &mut tx)
                .await?
                .unwrap();
        assert_eq!(oppgave_1.status, Opprettet);
        assert_eq!(oppgave_1.ekstern_oppgave_id, Some(100));
        assert!(
            oppgave_1
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveOpprettet)
        );

        // Oppgave 2: feilet — tilbake til Ubehandlet med feil-logg
        let oppgave_2 =
            hent_nyeste_oppgave(arbeidssoeker_id_2, OppgaveType::AvvistUnder18, &mut tx)
                .await?
                .unwrap();
        assert_eq!(oppgave_2.status, Ubehandlet);
        assert!(oppgave_2.ekstern_oppgave_id.is_none());
        assert!(
            oppgave_2
                .hendelse_logg
                .iter()
                .any(|logg| logg.status == EksternOppgaveOpprettelseFeilet)
        );

        // Oppgave 3: vellykket
        let oppgave_3 =
            hent_nyeste_oppgave(arbeidssoeker_id_3, OppgaveType::AvvistUnder18, &mut tx)
                .await?
                .unwrap();
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
    async fn to_tråder_prosesserer_samme_oppgave() -> Result<()> {
        let mut server = Server::new_async().await;
        let identitetsnummer = "12345678901";

        // Forventer 0 kall mot oppgaver apiet
        let oppgave_mock = server
            .mock("POST", OPPGAVER_PATH)
            .match_body(Matcher::Regex(format!(r#""personident":"{}"#, identitetsnummer)))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(json!({"id": 100, "tildeltEnhetsnr": "4863", "oppgavetype": "JFR", "tema": "KON", "prioritet": "NORM", "status": "OPPRETTET", "aktivDato": "2026-02-16", "versjon": 1}).to_string())
            .expect(0)
            .create_async()
            .await;

        let base_url = server.url();
        let oppgave_api_client = Arc::new(OppgaveApiClient::new(
            OppgaveClientConfig {
                base_url: base_url.into(),
                scope: "test-scope".to_string().into(),
            },
            Arc::new(StubTokenClient),
        ));

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = 12345;
        insert_oppgave(
            &InsertOppgaveRow {
                arbeidssoeker_id,
                identitetsnummer: identitetsnummer.to_string(),
                status: Ubehandlet.to_string(),
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        let mut hent_eldste_oppgaver_tx = pg_pool.begin().await?;
        let mut oppgaver = hent_de_eldste_ubehandlede_oppgavene(
            1,
            DateTime::UNIX_EPOCH,
            &mut hent_eldste_oppgaver_tx,
        )
        .await?;
        hent_eldste_oppgaver_tx.commit().await?;
        let oppgave = oppgaver.remove(0);

        let oppgave_id = oppgave.id;
        let mut tx_a = pg_pool.begin().await?;
        let resultat_a = bytt_oppgave_status(oppgave_id, Ubehandlet, Opprettet, &mut tx_a).await?;
        assert!(resultat_a, "Sett i gang row lock og hold den åpen");

        // Worker B: spawner prosesser_oppgave — blokkeres av Worker A sin row lock
        let pool_b = pg_pool.clone();
        let client_b = oppgave_api_client.clone();
        let worker_b =
            tokio::spawn(async move { prosesser_oppgave(&pool_b, &client_b, &oppgave).await });

        // Gi Worker B tid til å nå UPDATE og bli blokkert av row lock
        sleep(Duration::from_secs(1)).await;

        // Worker A committer — status er nå Opprettet, row lock frigjøres
        // Worker B sin CAS feiler (status er Opprettet, ikke Ubehandlet) og hopper over
        tx_a.commit().await?;

        let resultat_b = worker_b.await?;
        assert!(
            resultat_b.is_ok(),
            "Worker B skal returnere Ok (hoppet over pga CAS)"
        );

        // Verifiser at Worker B IKKE kalte det eksterne API-et
        oppgave_mock.assert_async().await;

        Ok(())
    }
}
