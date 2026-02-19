use crate::config::DatabaseConfig;
use crate::error::DatabaseError;
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};

async fn get_pg_pool(config: &DatabaseConfig) -> Result<PgPool> {
    let database_url = config.full_url();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)
        .map_err(DatabaseError::InitializePool)?;
    let _ = sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(DatabaseError::VerifyConnection)?;
    Ok(pool)
}

pub async fn init_db(config: DatabaseConfig) -> Result<PgPool> {
    log::info!("Database config: {:?}", config);
    let pg_pool = get_pg_pool(&config).await?;
    log::info!("Postgres pool opprettet");
    Ok(pg_pool)
}

/*
 * OBS! Denne funkejonen sletter alle tabeller i databasen
 */
pub async fn clear_db(pool: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, TableNameRow>(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
    )
    .fetch_all(pool)
    .await
    .map_err(DatabaseError::ExecuteQuery)?;
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
        .map_err(DatabaseError::ExecuteQuery)?;
    log::info!("Slettet alle tabeller i databasen");
    Ok(())
}

#[derive(Debug, FromRow)]
struct TableNameRow {
    pub tablename: String,
}
