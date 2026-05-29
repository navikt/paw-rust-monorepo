use crate::error::ConfigError;
use serde::Deserialize;

pub fn read_toml_config<'de, T: Deserialize<'de>>(content: &'de str) -> Result<T, ConfigError> {
    let config = toml::from_str::<T>(content)?;
    Ok(config)
}

#[macro_export]
macro_rules! read_config_file {
    ($cfg_name:expr) => {
        if cfg!(feature = "nais") {
            include_str!(concat!("../config/nais/", $cfg_name))
        } else {
            include_str!(concat!("../config/local/", $cfg_name))
        }
    };
}
