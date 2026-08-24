use paw_rdkafka_hwm::hwm_functions::{get_hwm, insert_hwm, update_hwm};
use postgres_testcontainer::postgres::setup_postgres_container;

#[tokio::test]
async fn test_hwm() {
    let postgres_guard = setup_postgres_container()
            .await
            .expect("Failed to start Postgres container");
    let pg_pool = postgres_guard.pg_pool;
    sqlx::migrate!("./migrations").run(&pg_pool).await.unwrap();
    
    let mut tx = pg_pool.begin().await.unwrap();
    assert!(get_hwm(&mut tx, 0, "A", 0).await.unwrap().is_none());
    assert!(get_hwm(&mut tx, 1, "A", 1).await.unwrap().is_none());
    assert!(insert_hwm(&mut tx, 0, "A", 0, 10).await.is_ok());
    assert_eq!(get_hwm(&mut tx, 0, "A", 0).await.unwrap().unwrap(), 10);
    assert!(get_hwm(&mut tx, 1, "A", 1).await.unwrap().is_none());
    assert!(update_hwm(&mut tx, 0, "A", 0, 15).await.unwrap());
    assert!(!update_hwm(&mut tx, 0, "A", 0, 15).await.unwrap());
    tx.commit().await.unwrap();
}
