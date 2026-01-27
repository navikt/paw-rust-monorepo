use crate::get_env::{get_env, AppError};
use log::info;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::error::Error;

pub struct DatabaseConfig {
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db_name: String,
    pub pg_ssl_cert_path: String,
    pub pg_ssl_key_path: String,
    pub pg_ssl_root_cert_path: String,
}

impl DatabaseConfig {
    pub fn full_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.user, self.password, self.ip, self.port, self.db_name
        )
    }
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("ip", &self.ip)
            .field("port", &self.port)
            .field("username", &self.user)
            .field("password", &"********")
            .field("db_name", &self.db_name)
            .field("pg_ssl_cert_path", &self.pg_ssl_cert_path)
            .field("pg_ssl_key_path", &self.pg_ssl_key_path)
            .field("pg_ssl_root_cert_path", &self.pg_ssl_root_cert_path)
            .finish()
    }
}

impl std::fmt::Display for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Database connection to {}:{}/{}",
            self.ip, self.port, self.db_name
        )
    }
}

pub fn get_database_config() -> Result<DatabaseConfig, AppError> {
    Ok(DatabaseConfig {
        ip: get_db_env("HOST")?,
        port: get_db_env("PORT")?.parse().map_err(|_| AppError {
            domain: "GET_ENV_VAR".to_string(),
            value: "PORT".to_string(),
        })?,
        user: get_db_env("USERNAME")?,
        password: get_db_env("PASSWORD")?,
        db_name: get_db_env("DATABASE")?,
        pg_ssl_cert_path: get_env("PGSSLCERT")?,
        pg_ssl_key_path: get_env("PGSSLKEY")?,
        pg_ssl_root_cert_path: get_env("PGSSLROOTCERT")?,
    })
}

fn get_db_env(var: &str) -> Result<String, AppError> {
    let key = format!(
        "NAIS_DATABASE_PAW_ARBEIDSSOEKERREGISTERET_AVVIST_TIL_OPPGAVE_AVVISTTILOPPGAVE_{}",
        var
    );
    std::env::var(&key).map_err(|_| AppError {
        domain: "GET_ENV_VAR".to_string(),
        value: format!("Failed to get env var {}", key),
    })
}

async fn get_pg_pool(config: &DatabaseConfig) -> Result<PgPool, AppError> {
    let database_url = config.full_url();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)
        .map_err(|_| AppError {
            domain: "DATABASE_CONNECTION".to_string(),
            value: "Failed to create PG Pool".to_string(),
        })?;
    let _ = sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| AppError {
            domain: "DATABASE_CONNECTION".to_string(),
            value: format!("Failed to run 'SELECT 1', connection not ok: {}", e),
        })?;
    Ok(pool)
}

pub async fn init_db() -> Result<PgPool, Box<dyn Error>> {
    let db_config = get_database_config()?;
    info!("Database paw_rust_base: {:?}", db_config);
    let pg_pool = get_pg_pool(&db_config).await?;
    info!("Postgres pool opprettet");
    let _ = sqlx::migrate!("./migrations").run(&pg_pool).await?;
    Ok(pg_pool)
}
