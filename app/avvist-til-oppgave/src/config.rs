use anyhow::Result;
use chrono::{DateTime, Utc};
use paw_app_config::config::read_toml_config;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rust_base::env;
use paw_sqlx::config::DatabaseConfig;
use serde::Deserialize;
use serde_env_field::env_field_wrap;
use texas_client::config::TokenClientConfig;

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct ApplicationConfig {
    pub topic_hendelseslogg: String,
    pub topic_hendelseslogg_version: i16,
    pub topic_oppgavehendelse: String,
    pub topic_oppgavehendelse_version: i16,
    pub opprett_oppgaver_task_interval_minutes: u64,
    pub opprett_oppgaver_task_batch_size: i64,
    pub opprett_oppgaver_fra_tidspunkt: DateTime<Utc>,
}

impl ApplicationConfig {
    pub fn topics_as_str(&self) -> Vec<&str> {
        vec![
            self.topic_hendelseslogg.as_str(),
            //self.topic_oppgavehendelse.as_str(),
        ]
    }
}

#[env_field_wrap]
#[derive(Debug, Clone, Deserialize)]
pub struct OppgaveClientConfig {
    pub base_url: String,
    pub scope: String,
}

pub fn read_application_config() -> Result<ApplicationConfig> {
    let file_content = read_application_config_file();
    read_toml_config::<ApplicationConfig>(file_content)
}

pub fn read_database_config() -> Result<DatabaseConfig> {
    let file_content = read_database_config_file();
    read_toml_config::<DatabaseConfig>(file_content)
}

pub fn read_kafka_config() -> Result<KafkaConfig> {
    let file_content = read_kafka_config_file();
    read_toml_config::<KafkaConfig>(file_content)
}

pub fn read_oppgave_client_config() -> Result<OppgaveClientConfig> {
    let file_content = read_oppgave_client_config_file();
    read_toml_config::<OppgaveClientConfig>(file_content)
}

pub fn read_token_client_config() -> Result<TokenClientConfig> {
    let file_content = read_token_client_config_file();
    let local: TokenClientConfigLocal = read_toml_config(file_content)?;
    Ok(TokenClientConfig {
        token_endpoint: local.token_endpoint.to_string(),
    })
}

#[env_field_wrap]
#[derive(Debug, Deserialize)]
struct TokenClientConfigLocal {
    token_endpoint: String,
}

fn read_application_config_file() -> &'static str {
    match env::nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/application_config.toml"),
        Err(_) => include_str!("../config/local/application_config.toml"),
    }
}

fn read_database_config_file() -> &'static str {
    match env::nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/database_config.toml"),
        Err(_) => include_str!("../config/local/database_config.toml"),
    }
}

fn read_kafka_config_file() -> &'static str {
    match env::nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/kafka_config.toml"),
        Err(_) => include_str!("../config/local/kafka_config.toml"),
    }
}

fn read_oppgave_client_config_file() -> &'static str {
    match env::nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/oppgave_client_config.toml"),
        Err(_) => include_str!("../config/local/oppgave_client_config.toml"),
    }
}

fn read_token_client_config_file() -> &'static str {
    match env::nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/token_client_config.toml"),
        Err(_) => include_str!("../config/local/token_client_config.toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_application_config() {
        let config = read_application_config().unwrap();
        println!("{:?}", config);
    }

    #[test]
    fn test_read_database_config() {
        let config = read_database_config().unwrap();
        println!("{:?}", config);
    }

    #[test]
    fn test_read_kafka_config() {
        let config = read_kafka_config().unwrap();
        println!("{:?}", config);
    }

    #[test]
    fn test_read_oppgave_client_config() {
        let config = read_oppgave_client_config().unwrap();
        println!("{:?}", config);
    }
}
