use serde::{Deserialize, Serialize};
use serde_env_field::env_field_wrap;

#[env_field_wrap]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClientConfig {
    pub token_endpoint: String,
    #[serde(default)]
    pub token_exchange_endpoint: Option<String>,
}
