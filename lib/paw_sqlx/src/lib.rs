use log::info;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::error::Error;
use std::fmt::format;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::env_var::{get_env, EnvVarNotFoundError};
use paw_rust_base::error_handling::AppError;
use paw_rust_base::nais_otel_service_name;

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

pub fn get_database_config(database_name: &'static str) -> Result<DatabaseConfig, Box<dyn AppError>> {
    Ok(DatabaseConfig {
        ip: get_db_env(database_name, "HOST")?,
        port: get_db_env(database_name, "PORT")?.parse().map_err(|err| {
            DatabaseError {
                message: format!("Invalid port number: {}", err)
            }
        })?,
        user: get_db_env(database_name, "USERNAME")?,
        password: get_db_env(database_name, "PASSWORD")?,
        db_name: get_db_env(database_name, "DATABASE")?,
        pg_ssl_cert_path: get_env("PGSSLCERT")?,
        pg_ssl_key_path: get_env("PGSSLKEY")?,
        pg_ssl_root_cert_path: get_env("PGSSLROOTCERT")?,
    })
}

fn get_db_env(database: &'static str, var: &'static str) -> Result<String, Box<dyn AppError>> {
    let service_name = nais_otel_service_name()?;
    let key = format!(
        "NAIS_DATABASE_{}_{}_{}",
        service_name,
        database,
        var
    );
    std::env::var(&key).map_err(|_| Box::from(EnvVarNotFoundError {
        env_var_name: database
    }))
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

pub async fn init_db(database_name: &'static str) -> Result<PgPool, Box<dyn AppError>> {
    let db_config = get_database_config(database_name)?;
    info!("Database paw_rust_base: {:?}", db_config);
    let pg_pool = get_pg_pool(&db_config).await?;
    info!("Postgres pool opprettet");
    Ok(pg_pool)
}
