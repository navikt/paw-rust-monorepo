use postgres_testcontainer::postgres::setup_postgres_container;
use rdkafka::message::{Header, OwnedHeaders, OwnedMessage, Timestamp};

#[tokio::test]
pub async fn test_basic_app_core_flow() {
    let postgres_guard = setup_postgres_container()
        .await
        .expect("Failed to start Postgres container");
    let pool = postgres_guard.pg_pool;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}
