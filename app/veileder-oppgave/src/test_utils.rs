use crate::config::OppgaveClientConfig;
use anyhow::Result;
use async_trait::async_trait;
use texas_client::response::TokenResponse;
use texas_client::token_client::M2MTokenClient;

pub(crate) struct MockTokenClient;

#[async_trait]
impl M2MTokenClient for MockTokenClient {
    async fn get_token(&self, _target: String) -> Result<TokenResponse> {
        Ok(TokenResponse {
            access_token: "dummy-token".to_string(),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }
}

pub(crate) fn test_client_config(base_url: String) -> OppgaveClientConfig {
    OppgaveClientConfig {
        base_url: base_url.into(),
        scope: "test-scope".to_string().into(),
    }
}
