use crate::error::ConfigError;
use anyhow::Result;
use serde::Deserialize;

pub fn read_toml_config<'de, T: Deserialize<'de>>(content: &'de str) -> Result<T> {
    let config = toml::from_str::<T>(content).map_err(|e| ConfigError::DeserializeTomlFile(e))?;
    Ok(config)
}
