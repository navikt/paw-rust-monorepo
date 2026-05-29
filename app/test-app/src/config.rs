use anyhow::Result;
use paw_app_config::config::read_toml_config;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rust_base::env;
use paw_sqlx::config::DatabaseConfig;

pub fn read_database_config() -> Result<DatabaseConfig> {
    let file_content = read_database_config_file();
    Ok(read_toml_config::<DatabaseConfig>(file_content)?)
}

pub fn read_kafka_config() -> Result<KafkaConfig> {
    let file_content = read_kafka_config_file();
    Ok(read_toml_config::<KafkaConfig>(file_content)?)
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
