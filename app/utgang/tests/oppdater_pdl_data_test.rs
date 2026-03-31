use std::num::NonZeroU16;
use std::sync::Arc;

use chrono::Duration;
use mockito::Server;
use paw_test::setup_test_db::setup_test_db;
use utgang::{
    db_read_ops::hent_opplysninger, db_write_ops::opprett_aktiv_periode,
    oppdater_pdl_data::PdlDataOppdatering, pdl::pdl_query::PDLClient, vo::kilde::InfoKilde,
};

mod common;

use crate::common::{
    StubTokenClient, lag_pdl_bolk_respons, lag_person_json, main_avro_periode,
    sett_gammel_sist_oppdatert,
};

async fn setup_db_med_periode(pool: &sqlx::PgPool, identitetsnummer: &str) -> uuid::Uuid {
    let periode = main_avro_periode();
    let periode_id = periode.id;

    let mut tx = pool.begin().await.expect("Failed to begin tx");
    opprett_aktiv_periode(&mut tx, &periode)
        .await
        .expect("Failed to insert periode");

    sqlx::query(
        "INSERT INTO periode_metadata (periode_id, identitetsnummer, arbeidssoeker_id, kafka_key) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode_id)
    .bind(identitetsnummer)
    .bind(1_i64)
    .bind(1_i64)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert periode_metadata");

    tx.commit().await.expect("Failed to commit");
    periode_id
}

#[tokio::test]
async fn kjoer_oppdatering_skriver_pdl_opplysninger() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let identitetsnummer = "12345678901";
    let periode_id = setup_db_med_periode(&pool, identitetsnummer).await;

    let mut tx = pool.begin().await.unwrap();
    sett_gammel_sist_oppdatert(&mut tx, &periode_id).await;
    tx.commit().await.unwrap();

    let person = lag_person_json("1970-01-01", "0301", "NOR", "bosattEtterFolkeregisterloven");
    let respons_body = lag_pdl_bolk_respons(vec![(identitetsnummer, Some(person))]);

    let mut pdl_server = Server::new_async().await;
    let _pdl_mock = pdl_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(respons_body)
        .create_async()
        .await;

    let pdl_client = PDLClient::new(
        "test-scope".to_string(),
        pdl_server.url(),
        reqwest::Client::new(),
        Arc::new(StubTokenClient),
    );

    let oppdatering = PdlDataOppdatering::new(
        pool.clone(),
        pdl_client,
        NonZeroU16::new(10).unwrap(),
        Duration::seconds(1),
    );

    oppdatering
        .kjoer_oppdatering()
        .await
        .expect("kjoer_oppdatering failed");

    let mut tx = pool.begin().await.unwrap();
    let opplysninger = hent_opplysninger(&mut tx, &periode_id, 10)
        .await
        .expect("Failed to fetch opplysninger");
    tx.commit().await.unwrap();

    let pdl_rad = opplysninger
        .iter()
        .find(|r| r.kilde == InfoKilde::PdlSjekk)
        .expect("Expected a PdlSjekk opplysninger row");

    assert!(
        !pdl_rad.opplysninger.is_empty(),
        "Expected non-empty opplysninger from PDL"
    );
    println!("PDL opplysninger: {:?}", pdl_rad.opplysninger);
}

#[tokio::test]
async fn kjoer_oppdatering_ingen_perioder_aa_oppdatere() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let identitetsnummer = "12345678902";
    let periode_id = setup_db_med_periode(&pool, identitetsnummer).await;

    let person = lag_person_json("1970-01-01", "0301", "NOR", "bosattEtterFolkeregisterloven");
    let respons_body = lag_pdl_bolk_respons(vec![(identitetsnummer, Some(person))]);

    let mut pdl_server = Server::new_async().await;
    let pdl_mock = pdl_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(respons_body)
        .expect(0)
        .create_async()
        .await;

    let pdl_client = PDLClient::new(
        "test-scope".to_string(),
        pdl_server.url(),
        reqwest::Client::new(),
        Arc::new(StubTokenClient),
    );

    let oppdatering = PdlDataOppdatering::new(
        pool.clone(),
        pdl_client,
        NonZeroU16::new(10).unwrap(),
        Duration::days(1),
    );

    oppdatering
        .kjoer_oppdatering()
        .await
        .expect("kjoer_oppdatering failed");

    pdl_mock.assert_async().await;

    let mut tx = pool.begin().await.unwrap();
    let opplysninger = hent_opplysninger(&mut tx, &periode_id, 10)
        .await
        .expect("Failed to fetch opplysninger");
    tx.commit().await.unwrap();

    assert!(
        opplysninger.iter().all(|r| r.kilde != InfoKilde::PdlSjekk),
        "Expected no PdlSjekk opplysninger row when period is fresh"
    );
}

#[tokio::test]
async fn kjoer_oppdatering_pdl_returnerer_ingen_person() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let identitetsnummer = "12345678903";
    let periode_id = setup_db_med_periode(&pool, identitetsnummer).await;

    let mut tx = pool.begin().await.unwrap();
    sett_gammel_sist_oppdatert(&mut tx, &periode_id).await;
    tx.commit().await.unwrap();

    let respons_body = lag_pdl_bolk_respons(vec![(identitetsnummer, None)]);

    let mut pdl_server = Server::new_async().await;
    let _pdl_mock = pdl_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(respons_body)
        .create_async()
        .await;

    let pdl_client = PDLClient::new(
        "test-scope".to_string(),
        pdl_server.url(),
        reqwest::Client::new(),
        Arc::new(StubTokenClient),
    );

    let oppdatering = PdlDataOppdatering::new(
        pool.clone(),
        pdl_client,
        NonZeroU16::new(10).unwrap(),
        Duration::seconds(1),
    );

    oppdatering
        .kjoer_oppdatering()
        .await
        .expect("kjoer_oppdatering should return Ok even when PDL has no person");

    let mut tx = pool.begin().await.unwrap();
    let opplysninger = hent_opplysninger(&mut tx, &periode_id, 10)
        .await
        .expect("Failed to fetch opplysninger");
    tx.commit().await.unwrap();

    assert!(
        opplysninger.iter().all(|r| r.kilde != InfoKilde::PdlSjekk),
        "Expected no PdlSjekk opplysninger when PDL returns null person"
    );
}
