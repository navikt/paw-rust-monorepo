use anyhow::Result;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;
use uuid::Uuid;

pub struct TestDbGuard;

static CONTAINER_PORT: OnceCell<u16> = OnceCell::const_new();

pub async fn setup_test_db() -> Result<(PgPool, TestDbGuard)> {
    let host_port = *CONTAINER_PORT
        .get_or_init(|| async {
            let container: &'static ContainerAsync<Postgres> = Box::leak(Box::new(
                Postgres::default()
                    .with_tag("18-alpine")
                    .start()
                    .await
                    .expect("Failed to start Postgres container"),
            ));
            container
                .get_host_port_ipv4(5432)
                .await
                .expect("Failed to get Postgres port")
        })
        .await;

    let schema = format!("test_{}", Uuid::new_v4().simple());

    let admin_url = format!(
        "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
        host_port
    );
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin_pool)
        .await?;
    admin_pool.close().await;

    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .port(host_port)
        .username("postgres")
        .password("postgres")
        .database("postgres")
        .options([("search_path", schema.as_str())]);

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(3)
        .connect_with(options)
        .await?;

    Ok((pool, TestDbGuard))
}

