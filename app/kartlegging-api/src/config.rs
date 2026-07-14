use paw_app_config::{config::read_toml_config, read_config_file};
use paw_date_time::duration;
use paw_key_gen_client::config::PawKeyGenClientConfig;
use paw_oauth2_resource_server::config::AuthConfig;
use paw_otel_tracing::config::OtelTracingConfig;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_sqlx::config::DatabaseConfig;
use pdl_client::pdl_config::PDLClientConfig;
use serde::Deserialize;
use std::time::Duration;
use texas_client::config::TokenClientConfig;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
pub struct AppConfig {
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub metrics_task_interval: Duration,
}

pub fn read_app_config() -> anyhow::Result<AppConfig> {
    let content = read_config_file!("app_config.toml");
    Ok(read_toml_config::<AppConfig>(content)?)
}

pub fn read_otel_tracing_config() -> anyhow::Result<OtelTracingConfig> {
    let content = read_config_file!("otel_tracing_config.toml");
    Ok(read_toml_config::<OtelTracingConfig>(content)?)
}

pub fn read_database_config() -> anyhow::Result<DatabaseConfig> {
    let content = read_config_file!("database_config.toml");
    Ok(read_toml_config::<DatabaseConfig>(content)?)
}

pub fn read_auth_config() -> anyhow::Result<AuthConfig> {
    let content = read_config_file!("auth_config.toml");
    Ok(read_toml_config::<AuthConfig>(content)?)
}

pub fn read_kafka_config() -> anyhow::Result<KafkaConfig> {
    let content = read_config_file!("kafka_config.toml");
    Ok(read_toml_config::<KafkaConfig>(content)?)
}

pub fn read_token_client_config() -> anyhow::Result<TokenClientConfig> {
    let content = read_config_file!("token_client_config.toml");
    Ok(read_toml_config::<TokenClientConfig>(content)?)
}

pub fn read_paw_key_gen_client_config() -> anyhow::Result<PawKeyGenClientConfig> {
    let content = read_config_file!("key_gen_client_config.toml");
    Ok(read_toml_config::<PawKeyGenClientConfig>(content)?)
}

pub fn read_pdl_client_config() -> anyhow::Result<PDLClientConfig> {
    let content = read_config_file!("pdl_client_config.toml");
    Ok(read_toml_config::<PDLClientConfig>(content)?)
}
