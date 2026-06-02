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
