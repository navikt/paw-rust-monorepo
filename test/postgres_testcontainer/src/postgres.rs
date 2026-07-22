use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;
use uuid::Uuid;

pub async fn setup_postgres_container(port: u16) -> anyhow::Result<TestContainerGuard> {
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
                .get_host_port_ipv4(port)
                .await
                .expect("Failed to get Postgres port")
        })
        .await;

    let admin_url = format!(
        "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
        host_port
    );
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;

    let schema = format!("test_{}", Uuid::new_v4().simple());
    let create_schema_sql = format!("CREATE SCHEMA {schema}");
    sqlx::query(AssertSqlSafe(create_schema_sql.as_str()))
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

    let pg_pool = PgPoolOptions::new()
        .min_connections(0)
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await?;

    Ok(TestContainerGuard { pg_pool })
}

static CONTAINER_PORT: OnceCell<u16> = OnceCell::const_new();

pub struct TestContainerGuard {
    pub pg_pool: PgPool,
}
