use async_trait::async_trait;
use texas_client::response::TokenResponse;
use texas_client::token_client::M2MTokenClient;

pub struct TokenClientStub;

impl TokenClientStub {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl M2MTokenClient for TokenClientStub {
    async fn get_token(&self, _target: String) -> anyhow::Result<TokenResponse> {
        Ok(TokenResponse {
            access_token: "stub-token".to_string(),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }
}
