use serde::Deserialize;
use paw_rust_base::env::nais_cluster_name;

pub const BEHANDLINGSNUMMER: &str = "B452";

#[derive(Deserialize, Debug)]
pub struct PDLClientConfig {
    pub target_scope: String,
    pub url: String
}

impl PDLClientConfig {
    pub fn new(target_scope: String, url: String) -> PDLClientConfig {
        PDLClientConfig {
            target_scope,
            url
        }
    }

    pub fn from_default_file() -> Result<Self, toml::de::Error> {
        let file_content = if nais_cluster_name().is_ok() {
            include_str!("../../config/nais/pdl_config.toml")
        } else {
            include_str!("../../config/local/pdl_config.toml")
        };
        toml::from_str(file_content)
    }
}
