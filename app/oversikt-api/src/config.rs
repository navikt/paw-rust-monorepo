use anyhow::Result;
use paw_app_config::{config::read_toml_config, read_config_file};
use paw_sqlx::config::DatabaseConfig;

pub fn read_database_config() -> Result<DatabaseConfig> {
    let content = read_config_file!("database_config.toml");
    Ok(read_toml_config::<DatabaseConfig>(content)?)
}
