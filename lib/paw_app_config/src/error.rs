use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("Failed to deserialize TOML config: {0}")]
    DeserializeTomlFile(#[from] toml::de::Error),
}
