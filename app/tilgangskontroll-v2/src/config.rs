use paw_oauth2_resource_server::oauth2::config::AuthConfig;
use paw_observability::otel::config::OtelTracingConfig;
use paw_observability::otel::format::OtelFormat;
use paw_rust_base::{config::read_toml_config, read_config_file};
use poao_tilgang_client::config::PoaoTilgangClientConfig;
use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::time::Duration;
use texas_client::config::TokenClientConfig;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

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

pub fn read_otel_tracing_config() -> anyhow::Result<OtelTracingConfig> {
    let content = read_config_file!("otel_tracing_config.toml");
    Ok(read_toml_config::<AppOtelTracingConfig>(content)?.as_config())
}

pub fn read_auth_config() -> anyhow::Result<AuthConfig> {
    let content = read_config_file!("auth_config.toml");
    Ok(read_toml_config::<AuthConfig>(content)?)
}

pub fn read_token_client_config() -> anyhow::Result<TokenClientConfig> {
    let content = read_config_file!("token_client_config.toml");
    Ok(read_toml_config::<TokenClientConfig>(content)?)
}

pub fn read_poao_tilgang_client_config() -> anyhow::Result<PoaoTilgangClientConfig> {
    let content = read_config_file!("poao_tilgang_client_config.toml");
    Ok(read_toml_config::<PoaoTilgangClientConfig>(content)?)
}
