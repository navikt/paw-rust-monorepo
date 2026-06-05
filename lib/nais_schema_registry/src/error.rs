#[derive(thiserror::Error, Debug)]
pub enum SchemaRegistryConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid schema registry URL: {0}")]
    InvalidUrl(String),
}
