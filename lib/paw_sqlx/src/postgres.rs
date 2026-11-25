use std::str::FromStr;

use crate::config::DatabaseConfig;
use anyhow::Result;
use errors::database::DatabaseError;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{AssertSqlSafe, ConnectOptions};
use sqlx::{FromRow, PgPool};

async fn get_pg_pool(config: &DatabaseConfig) -> Result<PgPool> {
    let database_url = config.full_url();
    let statement_log_level = config.statement_log_level();
    let slow_statement_threshold = config.slow_statement_threshold();
    let mut connect_options =
        PgConnectOptions::from_str(&database_url).map_err(DatabaseError::InitializePool)?;
    // sqlx faller stille tilbake til Prefer, uten sertifikatsjekk, når PGSSLMODE ikke kan leses.
    if let Some(ssl_mode) = ssl_mode(std::env::var("PGSSLMODE").ok().as_deref())
        .map_err(DatabaseError::InitializePool)?
    {
        connect_options = connect_options.ssl_mode(ssl_mode);
    }
    let connect_options = connect_options
        .log_statements(statement_log_level)
        .log_slow_statements(log::LevelFilter::Warn, slow_statement_threshold);
    let max_connections = config.max_connections.map(|v| *v).unwrap_or(5);
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_lazy_with(connect_options);
    let _ = sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(DatabaseError::VerifyConnection)?;
    Ok(pool)
}

fn ssl_mode(verdi: Option<&str>) -> Result<Option<PgSslMode>, sqlx::Error> {
    verdi.map(PgSslMode::from_str).transpose()
}

pub async fn init_db(config: &DatabaseConfig) -> Result<PgPool> {
    tracing::info!("Database config: {:?}", config);
    let pg_pool = get_pg_pool(config).await?;
    tracing::info!("Postgres pool opprettet");
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
    tracing::info!("Sletter tabellene: {}", tables.join(", "));
    let sql = format!("DROP TABLE IF EXISTS {} CASCADE", tables.join(", "));
    let _ = sqlx::query(AssertSqlSafe(sql.as_str()))
        .bind(tables.join(", ").as_str())
        .execute(pool)
        .await
        .map_err(DatabaseError::ExecuteQuery)?;
    tracing::info!("Slettet alle tabeller i databasen");
    Ok(())
}

#[derive(Debug, FromRow)]
struct TableNameRow {
    pub tablename: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ugyldig_pgsslmode_avvises_i_stedet_for_aa_bli_prefer() {
        assert!(ssl_mode(Some("VerifyFull")).is_err());
        assert!(matches!(
            ssl_mode(Some("verify-full")),
            Ok(Some(PgSslMode::VerifyFull))
        ));
        assert!(matches!(ssl_mode(None), Ok(None)));
    }
}
