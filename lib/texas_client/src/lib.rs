use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod error;
mod request;
pub mod token_client;

#[async_trait]
pub trait M2MTokenClient {
    async fn get_token(&self, target: String) -> Result<TokenResponse>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClientConfig {
    pub token_endpoint: String,
}
