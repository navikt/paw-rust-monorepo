use interne_hendelser::{Kilde, vo::Opplysning};
use paw_test::setup_test_db::setup_test_db;
use std::collections::HashSet;
use utgang::{
    db_read_ops::hent_opplysninger,
    db_write_ops::{self, skriv_pdl_info, skrive_startet_hendelse},
    kafka::periode_deserializer::{BrukerType, Metadata, Periode},
    vo::kilde::InfoKilde,
};
mod common;

use crate::common::{hendelse_startet, main_avro_periode};

#[tokio::test]
async fn test_db_migrations() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    let mut tx = pool.begin().await.expect("Failed to start transaction");
    let startet = hendelse_startet();
    skrive_startet_hendelse(&mut tx, &startet, 42)
        .await
        .expect("Failed to insert hendelse");
    tx.commit().await.expect("Failed to commit transaction");

    let mut tx = pool.begin().await.expect("Failed to start transaction");
    let opplysninger = hent_opplysninger(&mut tx, &startet.hendelse_id, 10)
        .await
        .expect("Failed to fetch opplysninger");
    tx.commit().await.expect("Failed to commit transaction");

    assert_eq!(opplysninger.len(), 1, "Expected 1 opplysninger for periode");
    let rad = opplysninger
        .first()
        .expect("Expected at least one opplysninger rad");
    assert_eq!(rad.kilde, InfoKilde::StartetHendelse, "Kilde should match");
    assert_eq!(
        rad.tidspunkt, startet.metadata.tidspunkt,
        "Tidspunkt should match"
    );
    let opplysninger_vec: Vec<Opplysning> = rad.opplysninger.to_vec();
    assert_eq!(
        rad.opplysninger, opplysninger_vec,
        "Opplysninger should match"
    );

    let periode = main_avro_periode();
    let mut tx = pool.begin().await.expect("Failed to start transaction");
    crate::db_write_ops::opprett_aktiv_periode(&mut tx, &periode)
        .await
        .expect("Failed to insert periode");
    tx.commit().await.expect("Failed to commit transaction");
    let mut tx = pool.begin().await.expect("Failed to start transaction");
    let result = crate::db_write_ops::avslutt_periode(
        &mut tx,
        &periode.id,
        &chrono::Utc::now(),
        &BrukerType::Sluttbruker,
    )
    .await
    .expect("Failed to update periode");
    assert!(
        result,
        "avslutt_periode should return true for successful update"
    );
    tx.commit().await.expect("Failed to commit transaction");
}

#[tokio::test]
async fn skrive_startet_hendelse_lagrer_korrekte_opplysninger() {
    let (pool, _container) = setup_test_db().await.expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let startet = hendelse_startet();
    let forventede: HashSet<Opplysning> = startet.opplysninger.clone();

    let mut tx = pool.begin().await.unwrap();
    skrive_startet_hendelse(&mut tx, &startet, 42)
        .await
        .expect("Failed to write startet hendelse");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_opplysninger(&mut tx, &startet.hendelse_id, 10)
        .await
        .expect("Failed to read opplysninger");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    let rad = rader.first().unwrap();
    assert_eq!(rad.kilde, InfoKilde::StartetHendelse);
    assert_eq!(rad.periode_id, startet.hendelse_id);
    let lest: HashSet<Opplysning> = rad.opplysninger.iter().cloned().collect();
    assert_eq!(lest, forventede, "Opplysninger read back should match what was written");
}

#[tokio::test]
async fn skriv_pdl_info_lagrer_korrekte_opplysninger() {
    let (pool, _container) = setup_test_db().await.expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let periode_id = uuid::Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO periode_metadata (periode_id, identitetsnummer, arbeidssoeker_id, kafka_key) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode_id)
    .bind("12345678901")
    .bind(1_i64)
    .bind(1_i64)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert periode_metadata");
    tx.commit().await.unwrap();

    let forventede = vec![
        Opplysning::ErOver18Aar,
        Opplysning::HarNorskAdresse,
        Opplysning::BosattEtterFregLoven,
    ];

    let mut tx = pool.begin().await.unwrap();
    skriv_pdl_info(&mut tx, &periode_id, forventede.clone())
        .await
        .expect("Failed to write pdl info");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_opplysninger(&mut tx, &periode_id, 10)
        .await
        .expect("Failed to read opplysninger");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    let rad = rader.first().unwrap();
    assert_eq!(rad.kilde, InfoKilde::PdlSjekk);
    assert_eq!(rad.periode_id, periode_id);
    let lest: HashSet<Opplysning> = rad.opplysninger.iter().cloned().collect();
    let forventet_set: HashSet<Opplysning> = forventede.into_iter().collect();
    assert_eq!(lest, forventet_set, "Opplysninger read back should match what was written");
}
