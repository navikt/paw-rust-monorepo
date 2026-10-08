use serde::Deserialize;
use serde_env_field::env_field_wrap;

#[env_field_wrap]
#[derive(Deserialize)]
pub struct PawTilgangskontrollClientConfig {
    pub url: String,
    pub target_scope: String,
}
