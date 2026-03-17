use anyhow::Result;
use paw_app_config::config::read_toml_config;
use serde::Deserialize;
use serde_env_field::env_field_wrap;

#[env_field_wrap]
#[derive(Debug, Clone, Deserialize)]
pub struct ReglerConfig {
    pub eea_land: Vec<String>,
}

impl ReglerConfig {
    pub fn eea_land_as_uppercase(&self) -> Vec<String> {
        self.eea_land
            .iter()
            .map(|land| land.to_uppercase())
            .collect()
    }
}

pub fn read_regler_config() -> Result<ReglerConfig> {
    let file_content = read_regler_config_file();
    read_toml_config::<ReglerConfig>(file_content)
}

fn read_regler_config_file() -> &'static str {
    include_str!("../../config/regler_config.toml")
}
