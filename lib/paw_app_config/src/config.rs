use crate::error::ConfigError;
use anyhow::Result;
use serde::Deserialize;

pub fn read_toml_config<'de, T: Deserialize<'de>>(content: &'de str) -> Result<T> {
    let config = toml::from_str::<T>(content).map_err(ConfigError::DeserializeTomlFile)?;
    Ok(config)
}

#[macro_export]
macro_rules! read_config_file {
    ($cfg_name:expr) => {
        if paw_rust_base::env::nais_cluster_name().is_ok() {
            include_str!(concat!("../config/nais/", $cfg_name))
        } else {
            include_str!(concat!("../config/local/", $cfg_name))
        }
    };
}
