use paw_kafka::kafka_config::KafkaConfig;
use paw_observability::otel::config::OtelTracingConfig;
use paw_observability::otel::format::OtelFormat;
use paw_rust_base::config::duration;
use paw_rust_base::{config::read_toml_config, read_config_file};
use paw_sqlx::config::DatabaseConfig;
use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::time::Duration;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub metrics_task_interval: Duration,
    pub kafka: AppKafkaConfig,
}

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct AppKafkaConfig {
    pub paw_periode_topic: String,
}

impl AppKafkaConfig {
    pub fn all_topics(&self) -> Vec<&str> {
        vec![self.paw_periode_topic.as_str()]
    }
}

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct AppOtelTracingConfig {
    pub format: String,
    pub directives: Vec<String>,
}

impl AppOtelTracingConfig {
    pub fn as_config(&self) -> OtelTracingConfig {
        let format = match self.format.clone().into_inner().to_uppercase().as_str() {
            "JSON" => OtelFormat::Json,
            "OTEL_JSON" => OtelFormat::OtelJson,
            "FULL" => OtelFormat::Full,
            "PRETTY" => OtelFormat::Pretty,
            "COMPACT" => OtelFormat::Compact,
            _ => OtelFormat::default(),
        };
        let directives = self
            .directives
            .iter()
            .map(|c| c.clone().into_inner())
            .collect();
        OtelTracingConfig { format, directives }
    }
}

pub fn read_app_config() -> anyhow::Result<AppConfig> {
    let content = read_config_file!("app_config.toml");
    Ok(read_toml_config::<AppConfig>(content)?)
}

pub fn read_database_config() -> anyhow::Result<DatabaseConfig> {
    let content = read_config_file!("database_config.toml");
    Ok(read_toml_config::<DatabaseConfig>(content)?)
}

pub fn read_kafka_config() -> anyhow::Result<KafkaConfig> {
    let content = read_config_file!("kafka_config.toml");
    Ok(read_toml_config::<KafkaConfig>(content)?)
}

pub fn read_otel_tracing_config() -> anyhow::Result<OtelTracingConfig> {
    let content = read_config_file!("otel_tracing_config.toml");
    Ok(read_toml_config::<AppOtelTracingConfig>(content)?.as_config())
}
