use crate::config::DatabaseConfig;
use log::info;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::error_handling::AppError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

async fn get_pg_pool(config: &DatabaseConfig) -> Result<PgPool, Box<dyn AppError>> {
    let database_url = config.full_url();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)
        .map_err(|_| DatabaseError {
            message: "Failed to create Postgres connection pool".to_string(),
        })?;
    let _ = sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| DatabaseError {
            message: format!("Failed to run 'SELECT 1', connection not ok: {}", e),
        })?;
    Ok(pool)
}

pub async fn init_db(config: DatabaseConfig) -> Result<PgPool, Box<dyn AppError>> {
    info!("Database config: {:?}", config);
    let pg_pool = get_pg_pool(&config).await?;
    info!("Postgres pool opprettet");
    Ok(pg_pool)
}
