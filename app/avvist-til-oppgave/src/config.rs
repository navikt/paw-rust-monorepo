use paw_app_config::config::read_toml_config;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rust_base::error_handling::AppError;
use paw_rust_base::nais_cluster_name;
use paw_sqlx::DatabaseConfig;
use serde::Deserialize;
use serde_env_field::env_field_wrap;

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct ApplicationConfig {
    pub hendelselogg_topic: String,
}

pub fn read_application_config() -> Result<ApplicationConfig, Box<dyn AppError>> {
    let file_content = read_application_config_file();
    read_toml_config::<ApplicationConfig>(file_content)
}

pub fn read_database_config() -> Result<DatabaseConfig, Box<dyn AppError>> {
    let file_content = read_database_config_file();
    read_toml_config::<DatabaseConfig>(file_content)
}

pub fn read_kafka_config() -> Result<KafkaConfig, Box<dyn AppError>> {
    let file_content = read_kafka_config_file();
    read_toml_config::<KafkaConfig>(file_content)
}

fn read_application_config_file() -> &'static str {
    match nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/application_config.toml"),
        Err(_) => include_str!("../config/local/application_config.toml"),
    }
}

fn read_database_config_file() -> &'static str {
    match nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/database_config.toml"),
        Err(_) => include_str!("../config/local/database_config.toml"),
    }
}

fn read_kafka_config_file() -> &'static str {
    match nais_cluster_name() {
        Ok(_) => include_str!("../config/nais/kafka_config.toml"),
        Err(_) => include_str!("../config/local/kafka_config.toml"),
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
}
