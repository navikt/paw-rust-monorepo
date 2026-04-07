use interne_hendelser::vo::Opplysning;
use paw_test::setup_test_db::setup_test_db;
use std::collections::HashSet;
use std::num::NonZeroU16;
use utgang::{
    db_read_ops::{hent_opplysninger, hent_sist_oppdatert_foer_med_metadata},
    db_write_ops::{
        self, avslutt_periode, opprett_aktiv_periode, skriv_pdl_info_batch, skrive_startet_hendelse,
    },
    kafka::periode_deserializer::BrukerType,
    vo::{kilde::InfoKilde, status::Status},
};
mod common;

use crate::common::{hendelse_startet, main_avro_periode, sett_gammel_sist_oppdatert};

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
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
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
    assert_eq!(
        lest, forventede,
        "Opplysninger read back should match what was written"
    );
}

#[tokio::test]
async fn skriv_pdl_info_lagrer_korrekte_opplysninger() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
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
    skriv_pdl_info_batch(&mut tx, vec![(periode_id, forventede.clone())])
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
    assert_eq!(
        lest, forventet_set,
        "Opplysninger read back should match what was written"
    );
}

#[tokio::test]
async fn hent_sist_oppdatert_foer_med_metadata_returnerer_kun_rader_med_match_i_begge_tabeller() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let grense = chrono::Utc::now();
    let limit = NonZeroU16::new(10).unwrap();

    // Periode with matching metadata, backdated -> should be returned
    let periode_med_metadata = main_avro_periode();
    let mut tx = pool.begin().await.unwrap();
    opprett_aktiv_periode(&mut tx, &periode_med_metadata)
        .await
        .expect("Failed to insert periode");
    sqlx::query(
        "INSERT INTO periode_metadata (periode_id, identitetsnummer, arbeidssoeker_id, kafka_key) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode_med_metadata.id)
    .bind("12345678901")
    .bind(99_i64)
    .bind(42_i64)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert periode_metadata");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    sett_gammel_sist_oppdatert(&mut tx, &periode_med_metadata.id).await;
    tx.commit().await.unwrap();

    // Periode without metadata, backdated -> should NOT be returned (no JOIN match)
    let periode_uten_metadata = main_avro_periode();
    let mut tx = pool.begin().await.unwrap();
    opprett_aktiv_periode(&mut tx, &periode_uten_metadata)
        .await
        .expect("Failed to insert periode");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    sett_gammel_sist_oppdatert(&mut tx, &periode_uten_metadata.id).await;
    tx.commit().await.unwrap();

    // Periode with metadata but NOT backdated -> should NOT be returned (timestamp after grense)
    let periode_ny = main_avro_periode();
    let mut tx = pool.begin().await.unwrap();
    opprett_aktiv_periode(&mut tx, &periode_ny)
        .await
        .expect("Failed to insert periode");
    sqlx::query(
        "INSERT INTO periode_metadata (periode_id, identitetsnummer, arbeidssoeker_id, kafka_key) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode_ny.id)
    .bind("11111111111")
    .bind(100_i64)
    .bind(43_i64)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert periode_metadata");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_sist_oppdatert_foer_med_metadata(&mut tx, &grense, &[Status::Ok], &limit)
        .await
        .expect("Failed to read rader");
    tx.commit().await.unwrap();

    assert_eq!(
        rader.len(),
        1,
        "Skal kun returnere periode som har match i begge tabeller og er eldre enn grensen"
    );
    let rad = rader.first().unwrap();
    assert_eq!(rad.id, periode_med_metadata.id);
    assert_eq!(rad.identitetsnummer, "12345678901");
    assert_eq!(rad.arbeidssoeker_id, 99_i64);
    assert_eq!(rad.kafka_key, 42_i64);
}

#[tokio::test]
async fn hent_sist_oppdatert_foer_med_metadata_ekskluderer_avsluttede_perioder() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let grense = chrono::Utc::now();
    let limit = NonZeroU16::new(10).unwrap();

    let periode = main_avro_periode();
    let mut tx = pool.begin().await.unwrap();
    opprett_aktiv_periode(&mut tx, &periode)
        .await
        .expect("Failed to insert periode");
    sqlx::query(
        "INSERT INTO periode_metadata (periode_id, identitetsnummer, arbeidssoeker_id, kafka_key) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode.id)
    .bind("12345678901")
    .bind(1_i64)
    .bind(1_i64)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert periode_metadata");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    sett_gammel_sist_oppdatert(&mut tx, &periode.id).await;
    avslutt_periode(
        &mut tx,
        &periode.id,
        &chrono::Utc::now(),
        &BrukerType::Sluttbruker,
    )
    .await
    .expect("Failed to avslutt periode");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_sist_oppdatert_foer_med_metadata(&mut tx, &grense, &[Status::Ok], &limit)
        .await
        .expect("Failed to read rader");
    tx.commit().await.unwrap();

    assert!(rader.is_empty(), "Avsluttede perioder skal ikke returneres");
}
