use anyhow::Result;
use paw_app_config::{config::read_toml_config, read_config_file};
use paw_sqlx::config::DatabaseConfig;
use paw_texas_resource_server::config::AuthConfig;

pub fn read_database_config() -> Result<DatabaseConfig> {
    let content = read_config_file!("database_config.toml");
    Ok(read_toml_config::<DatabaseConfig>(content)?)
}

pub fn read_auth_config() -> Result<AuthConfig> {
    let content = read_config_file!("auth_config.toml");
    read_toml_config::<AuthConfig>(content).map_err(Into::into)
}
