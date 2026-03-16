use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClientConfig {
    pub token_endpoint: String,
    #[serde(default)]
    pub token_exchange_endpoint: Option<String>,
}
