use paw_test::setup_test_db::setup_test_db;
use utgang::{
    db_write_ops,
    kafka::periode_deserializer::{BrukerType, Metadata, Periode},
};
mod common;

use crate::common::main_avro_periode;

#[tokio::test]
async fn test_db_migrations() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

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
