use paw_rust_base::env::nais_cluster_name;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchemaRegistryConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid schema registry URL: {0}")]
    InvalidUrl(String),
}

pub fn create_nais_schema_registry_settings() -> Result<SrSettings, SchemaRegistryConfigError> {
    let schema_registry_url = env::var("KAFKA_SCHEMA_REGISTRY").map_err(|_| {
        SchemaRegistryConfigError::MissingEnvVar("KAFKA_SCHEMA_REGISTRY".to_string())
    })?;

    let username = env::var("KAFKA_SCHEMA_REGISTRY_USER").map_err(|_| {
        SchemaRegistryConfigError::MissingEnvVar("KAFKA_SCHEMA_REGISTRY_USER".to_string())
    })?;

    let password = env::var("KAFKA_SCHEMA_REGISTRY_PASSWORD").map_err(|_| {
        SchemaRegistryConfigError::MissingEnvVar("KAFKA_SCHEMA_REGISTRY_PASSWORD".to_string())
    })?;

    let sr_settings = SrSettings::new_builder(schema_registry_url)
        .set_basic_authorization(&username, Some(&password))
        .build()
        .map_err(|e| SchemaRegistryConfigError::InvalidUrl(e.to_string()))?;

    Ok(sr_settings)
}

pub fn create_local_schema_registry_settings() -> SrSettings {
    let schema_registry_url =
        env::var("KAFKA_SCHEMA_REGISTRY").unwrap_or_else(|_| "http://localhost:8081".to_string());
    SrSettings::new(schema_registry_url)
}

pub fn create_schema_registry_settings() -> Result<SrSettings, SchemaRegistryConfigError> {
    if nais_cluster_name().is_ok() {
        create_nais_schema_registry_settings()
    } else {
        Ok(create_local_schema_registry_settings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_env;

    #[test]
    fn test_create_local_schema_registry_settings() {
        temp_env::with_var_unset("NAIS_CLUSTER_NAME", || {
            let result = create_schema_registry_settings();
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_create_nais_schema_registry_settings_missing_vars() {
        temp_env::with_var("NAIS_CLUSTER_NAME", Some("dev-gcp"), || {
            temp_env::with_var_unset("KAFKA_SCHEMA_REGISTRY", || {
                let result = create_schema_registry_settings();
                assert!(result.is_err());
            });
        });
    }

    #[test]
    fn test_create_nais_schema_registry_settings_with_vars() {
        temp_env::with_vars(
            [
                ("NAIS_CLUSTER_NAME", Some("dev-gcp")),
                (
                    "KAFKA_SCHEMA_REGISTRY",
                    Some("https://kafka-schema-registry.nais.io"),
                ),
                ("KAFKA_SCHEMA_REGISTRY_USER", Some("test-user")),
                ("KAFKA_SCHEMA_REGISTRY_PASSWORD", Some("test-password")),
            ],
            || {
                let result = create_schema_registry_settings();
                assert!(result.is_ok());
            },
        );
    }
}
