use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::str::FromStr;

#[env_field_wrap]
#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub statement_log_level: Option<String>
}

impl DatabaseConfig {
    pub fn full_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }

    pub fn statement_log_level(&self) -> log::LevelFilter {
        match self.statement_log_level.as_deref() {
            Some(level) => log::LevelFilter::from_str(level).unwrap_or(log::LevelFilter::Debug),
            None => log::LevelFilter::Debug,
        }
    }
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"********")
            .field("database", &self.database)
            .finish()
    }
}

impl std::fmt::Display for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Database connection to {}:{}/{}",
            self.host, self.port, self.database
        )
    }
}
