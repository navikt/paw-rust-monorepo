use crate::config_error::ConfigError;
use paw_rust_base::error_handling::AppError;
use serde::Deserialize;

pub fn read_toml_config<'de, T: Deserialize<'de>>(
    content: &'de str,
) -> Result<T, Box<dyn AppError>> {
    toml::from_str::<T>(content).map_err(|e| {
        Box::new(ConfigError {
            message: format!("Failed to deserialize config: {}", e),
        }) as Box<dyn AppError>
    })
}
