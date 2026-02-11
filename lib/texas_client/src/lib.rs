use paw_rust_base::error_handling::AppError;
use serde::{Deserialize, Serialize};

mod request;
pub mod texas_error;
mod token_client;

trait M2MTokenClient {
    async fn get_token(&self, target: String) -> Result<TokenResponse, Box<dyn AppError>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}
