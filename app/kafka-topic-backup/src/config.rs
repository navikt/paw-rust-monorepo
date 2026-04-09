use paw_rdkafka::kafka_config::KafkaConfig;
use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::error::Error;

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct Config {
    pub topics: Vec<String>,
}

impl Config {
    pub fn from_string(file_content: &str) -> Result<Self, Box<dyn Error>> {
        let config: Config = toml::from_str(file_content)?;
        Ok(config)
    }

    pub fn from_default_file() -> Result<Self, Box<dyn Error>> {
        let file_content = include_str!("../config/config.toml");
        Self::from_string(file_content)
    }

    pub fn topics_as_str_slice(&self) -> Vec<&str> {
        self.topics.iter().map(|s| s.as_str()).collect()
    }
}

pub fn read_kafka_config() -> Result<KafkaConfig, Box<dyn Error>> {
    let file_content = match std::env::var("NAIS_CLUSTER_NAME") {
        Ok(_) => include_str!("../config/nais/kafka_config.toml"),
        Err(_) => include_str!("../config/local/kafka_config.toml"),
    };
    let config: KafkaConfig = toml::from_str(file_content)?;
    Ok(config)
}
