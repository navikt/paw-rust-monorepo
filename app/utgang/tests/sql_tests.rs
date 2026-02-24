use interne_hendelser::{Kilde, vo::Opplysning};
use paw_test::setup_test_db::setup_test_db;
use utgang::{
    db_read_ops::hent_opplysninger,
    db_write_ops::{self, skrive_startet_hendelse},
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
