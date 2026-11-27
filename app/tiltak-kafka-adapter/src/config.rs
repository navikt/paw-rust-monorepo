use chrono::{DateTime, Utc};
use mulighetsrommet_client::config::MulighetsrommetClientConfig;
use paw_kafka::kafka_config::KafkaConfig;
use paw_key_gen_client::config::PawKeyGenClientConfig;
use paw_observability::otel::config::OtelTracingConfig;
use paw_observability::otel::format::OtelFormat;
use paw_rust_base::config::duration;
use paw_rust_base::{config::read_toml_config, read_config_file};
use paw_sqlx::config::DatabaseConfig;
use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::time::Duration;
use texas_client::config::TokenClientConfig;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub metrics_task_interval: Duration,
    pub synk: SynkConfig,
    pub kafka: AppKafkaConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SynkConfig {
    pub aktivert: bool,
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub task_interval: Duration,
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub synk_interval: Duration,
    pub batch_size: i64,
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub batch_grace_time: Duration,
    #[serde(deserialize_with = "duration::iso8601::deserialize")]
    pub request_grace_time: Duration,
    pub maks_feil_per_rad: i32,
    pub maks_feil_per_kjoering: u32,
}

impl SynkConfig {
    pub fn synk_interval_grensed(&self) -> anyhow::Result<DateTime<Utc>> {
        Ok(Utc::now() - chrono::Duration::from_std(self.synk_interval)?)
    }
}

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct AppKafkaConfig {
    pub paw_periode_topic: String,
    pub paw_arbeidsmarkedstiltak_topic: String,
}

impl AppKafkaConfig {
    pub fn consumer_topics(&self) -> Vec<&str> {
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

pub fn read_token_client_config() -> anyhow::Result<TokenClientConfig> {
    let content = read_config_file!("token_client_config.toml");
    Ok(read_toml_config::<TokenClientConfig>(content)?)
}

pub fn read_paw_key_gen_client_config() -> anyhow::Result<PawKeyGenClientConfig> {
    let content = read_config_file!("paw_key_gen_client_config.toml");
    Ok(read_toml_config::<PawKeyGenClientConfig>(content)?)
}

pub fn read_mulighetsrommet_client_config() -> anyhow::Result<MulighetsrommetClientConfig> {
    let content = read_config_file!("mulighetsrommet_client_config.toml");
    Ok(read_toml_config::<MulighetsrommetClientConfig>(content)?)
}
