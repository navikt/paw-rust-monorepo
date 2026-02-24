use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClientConfig {
    pub token_endpoint: String,
}
