use anyhow::Result;
use async_trait::async_trait;
use texas_client::response::TokenResponse;
use texas_client::token_client::M2MTokenClient;

pub struct StubTokenClient;

#[async_trait]
impl M2MTokenClient for StubTokenClient {
    async fn get_token(&self, _target: String) -> Result<TokenResponse> {
        Ok(TokenResponse {
            access_token: "stub-token".to_string(),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }
}
