use serde::Deserialize;
use serde_env_field::env_field_wrap;

pub const BEHANDLINGSNUMMER: &str = "B452";

#[env_field_wrap]
#[derive(Deserialize, Debug)]
pub struct PDLClientConfig {
    pub target_scope: String,
    pub url: String,
}

impl PDLClientConfig {
    pub fn new(target_scope: String, url: String) -> PDLClientConfig {
        PDLClientConfig {
            target_scope: target_scope.into(),
            url: url.into(),
        }
    }
}
