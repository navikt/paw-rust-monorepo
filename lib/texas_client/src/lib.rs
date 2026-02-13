use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

mod request;
pub mod error;
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
