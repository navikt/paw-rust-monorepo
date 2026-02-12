use crate::config::DatabaseConfig;
use log::info;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::error_handling::AppError;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};

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

/*
 * OBS! Denne funkejonen sletter alle tabeller i databasen
 */
pub async fn clear_db(pool: &PgPool) -> Result<(), Box<dyn AppError>> {
    let rows = sqlx::query_as::<_, TableNameRow>(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DatabaseError {
        message: format!("Failed to get table list, connection not ok: {}", e),
    })?;
    let tables = rows
        .iter()
        .map(|row| row.tablename.as_str())
        .collect::<Vec<&str>>();
    log::info!("Sletter tabellene: {}", tables.join(", "));
    let sql = format!("DROP TABLE IF EXISTS {} CASCADE", tables.join(", "));
    let _ = sqlx::query(sql.as_str())
        .bind(tables.join(", ").as_str())
        .execute(pool)
        .await
        .map_err(|e| DatabaseError {
            message: format!("Failed to clear database, connection not ok: {}", e),
        })?;
    log::info!("Slettet alle tabeller i databasen");
    Ok(())
}

#[derive(Debug, FromRow)]
struct TableNameRow {
    pub tablename: String,
}
