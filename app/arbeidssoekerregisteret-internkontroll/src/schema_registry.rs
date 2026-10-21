use paw_rust_base::env::{RuntimeEnv, nais_cluster_name};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::env;

#[derive(Debug, thiserror::Error)]
pub enum SchemaRegistryConfigError {
    #[error("missing environment variable {0}")]
    MissingEnvVar(&'static str),
    #[error("invalid schema registry settings: {0}")]
    Invalid(String),
}

fn env_var(name: &'static str) -> Result<String, SchemaRegistryConfigError> {
    env::var(name).map_err(|_| SchemaRegistryConfigError::MissingEnvVar(name))
}

/// Nais provides the schema registry credentials through the Kafka pool. Locally the
/// registry runs without authentication.
pub fn create_schema_registry_settings(
    runtime_env: &RuntimeEnv,
) -> Result<SrSettings, SchemaRegistryConfigError> {
    match runtime_env {
        RuntimeEnv::Local => {
            let url = env::var("KAFKA_SCHEMA_REGISTRY")
                .unwrap_or_else(|_| "http://localhost:8081".into());
            Ok(SrSettings::new(url))
        }
        _ => {
            let password = env_var("KAFKA_SCHEMA_REGISTRY_PASSWORD")?;
            SrSettings::new_builder(env_var("KAFKA_SCHEMA_REGISTRY")?)
                .set_basic_authorization(&env_var("KAFKA_SCHEMA_REGISTRY_USER")?, Some(&password))
                .build()
                .map_err(|e| SchemaRegistryConfigError::Invalid(e.to_string()))
        }
    }
}
