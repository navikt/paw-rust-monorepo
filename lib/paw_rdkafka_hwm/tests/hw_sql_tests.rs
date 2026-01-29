use sqlx::{PgPool, Postgres as SqlxPostgres};
use std::error::Error;
use rdkafka::statistics::TopicPartition;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use paw_rdkafka_hwm::hwm_functions::{get_hwm, insert_hwm, update_hwm};

async fn setup_test_db() -> Result<(PgPool, ContainerAsync<Postgres>), Box<dyn Error>> {
    let postgres_container = Postgres::default()
        .with_tag("18-alpine")
        .start().await?;

    let host_port = postgres_container.get_host_port_ipv4(5432).await?;
    let connection_string = format!(
        "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
        host_port
    );

    // Set environment variables for testing (wrapped in unsafe blocks)
    unsafe {
        std::env::set_var("DATABASE_URL", &connection_string);
        std::env::set_var("PG_HOST", "127.0.0.1");
        std::env::set_var("PG_PORT", host_port.to_string());
        std::env::set_var("PG_USERNAME", "postgres");
        std::env::set_var("PG_PASSWORD", "postgres");
        std::env::set_var("PG_DATABASE_NAME", "postgres");
    }

    // Create connection pool and tables manually since init_db might not work in tests
    let pool = PgPool::connect(&connection_string).await?;

    // Create tables manually - need to import the SQL constant
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok((pool, postgres_container))
}

#[tokio::test]
async fn test_hwm() {
    let (pg_pool, _) = setup_test_db().await.unwrap();
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