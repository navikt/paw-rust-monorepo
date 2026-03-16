use super::client::{parse_token_response, request_send_error, ReqwestTokenClient};
use crate::request::M2MTokenRequest;
use crate::response::TokenResponse;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait M2MTokenClient {
    async fn get_token(&self, target: String) -> Result<TokenResponse>;
}

#[async_trait]
impl M2MTokenClient for ReqwestTokenClient {
    async fn get_token(&self, target: String) -> Result<TokenResponse> {
        let request = M2MTokenRequest::new(target.clone());

        let response = self
            .inner
            .client
            .post(&self.inner.token_endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| request_send_error(&e, target.clone()))?;

        parse_token_response(response, target).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TokenClientConfig;
    use crate::token_client::client::create_token_client;
    use mockito::Server;
    use reqwest::Client;
    use serde_json::json;

    fn create_test_client(base_url: String) -> ReqwestTokenClient {
        let config = TokenClientConfig {
            token_endpoint: format!("{}/api/v1/token", base_url),
            token_exchange_endpoint: None,
        };
        create_token_client(config, Client::new())
    }

    #[tokio::test]
    async fn test_hent_token_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "et-gyldig-token",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let result = client
            .get_token("api://test-scope/.default".to_string())
            .await;

        assert!(result.is_ok());
        let token = result.unwrap();
        assert_eq!(token.access_token, "et-gyldig-token");
        assert_eq!(token.expires_in, 3600);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_hent_token_feil_gir_token_error() {
        let mut server = Server::new_async().await;

        let forventet_error = json!({
            "error": "unauthorized_client",
            "error_description": "The client is not authorized"
        });

        let mock = server
            .mock("POST", "/api/v1/token")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(forventet_error.to_string())
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let error = client
            .get_token("api://test-scope/.default".to_string())
            .await
            .unwrap_err();

        let melding = error.to_string();
        assert!(
            melding.contains(forventet_error["error"].as_str().unwrap()),
            "{}",
            melding
        );
        assert!(
            melding.contains(forventet_error["error_description"].as_str().unwrap()),
            "{}",
            melding
        );
        mock.assert_async().await;
    }
}
