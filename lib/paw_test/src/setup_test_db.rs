use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

pub async fn setup_test_db() -> Result<(PgPool, ContainerAsync<Postgres>)> {
    let postgres_container = Postgres::default().with_tag("18-alpine").start().await?;

    let host_port = postgres_container.get_host_port_ipv4(5432).await?;
    let connection_string = format!(
        "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
        host_port
    );

    unsafe {
        std::env::set_var("DATABASE_URL", &connection_string);
        std::env::set_var("PG_HOST", "127.0.0.1");
        std::env::set_var("PG_PORT", host_port.to_string());
        std::env::set_var("PG_USERNAME", "postgres");
        std::env::set_var("PG_PASSWORD", "postgres");
        std::env::set_var("PG_DATABASE_NAME", "postgres");
    }

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(3)
        .connect(&connection_string)
        .await?;

    Ok((pool, postgres_container))
}
