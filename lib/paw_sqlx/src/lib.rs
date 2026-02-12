use log::info;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::error_handling::AppError;
use serde::Deserialize;
use serde_env_field::env_field_wrap;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[env_field_wrap]
#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
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
            self.username, self.password, self.host, self.port, self.db_name
        )
    }
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
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
            self.host, self.port, self.db_name
        )
    }
}

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
