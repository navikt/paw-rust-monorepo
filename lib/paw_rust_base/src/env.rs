use crate::error::ServerError;
use anyhow::Result;

pub fn get_env(key: &'static str) -> Result<String> {
    let var = std::env::var(key).map_err(|_| ServerError::EnvVarNotFound(key.to_string()))?;
    Ok(var)
}

pub fn nais_otel_service_name() -> Result<String> {
    get_env("OTEL_SERVICE_NAME")
}

pub fn nais_namespace() -> Result<String> {
    get_env("NAIS_NAMESPACE")
}

pub fn nais_cluster_name() -> Result<String> {
    get_env("NAIS_CLUSTER_NAME")
}
