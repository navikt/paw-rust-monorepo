use log::info;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::env_var::{get_env, EnvVarNotFoundError};
use paw_rust_base::error_handling::AppError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

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

pub fn get_database_config(env_prefix: &'static str) -> Result<DatabaseConfig, Box<dyn AppError>> {
    Ok(DatabaseConfig {
        ip: get_db_env(env_prefix, "HOST")?,
        port: get_db_env(env_prefix, "PORT")?
            .parse()
            .map_err(|err| DatabaseError {
                message: format!("Invalid port number: {}", err),
            })?,
        user: get_db_env(env_prefix, "USERNAME")?,
        password: get_db_env(env_prefix, "PASSWORD")?,
        db_name: get_db_env(env_prefix, "DATABASE")?,
        pg_ssl_cert_path: get_db_env(env_prefix, "SSLCERT")?,
        pg_ssl_key_path: get_db_env(env_prefix, "SSLKEY")?,
        pg_ssl_root_cert_path: get_db_env(env_prefix, "SSLROOTCERT")?,
    })
}

fn get_db_env(
    env_prefix: &'static str,
    env_var: &'static str,
) -> Result<String, Box<dyn AppError>> {
    let key = format!("{}_{}", env_prefix, env_var);
    std::env::var(&key).map_err(|_| {
        Box::from(EnvVarNotFoundError {
            env_var_name: env_var,
        })
    })
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

pub async fn init_db(env_prefix: &'static str) -> Result<PgPool, Box<dyn AppError>> {
    let db_config = get_database_config(env_prefix)?;
    info!("Database paw_rust_base: {:?}", db_config);
    let pg_pool = get_pg_pool(&db_config).await?;
    info!("Postgres pool opprettet");
    Ok(pg_pool)
}
