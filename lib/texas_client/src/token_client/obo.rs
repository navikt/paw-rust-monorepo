use super::client::{parse_token_response, request_send_error, ReqwestTokenClient};
use crate::error::TexasClientError;
use crate::request::OBOTokenRequest;
use crate::response::TokenResponse;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait OBOTokenClient {
    async fn exchange_token(&self, user_token: String, target: String) -> Result<TokenResponse>;
}

#[async_trait]
impl OBOTokenClient for ReqwestTokenClient {
    async fn exchange_token(&self, user_token: String, target: String) -> Result<TokenResponse> {
        let exchange_endpoint = self
            .inner
            .token_exchange_endpoint
            .as_deref()
            .ok_or(TexasClientError::ExchangeEndpointNotConfigured)?;

        let request = OBOTokenRequest::new(user_token, target.clone());

        let response = self
            .inner
            .client
            .post(exchange_endpoint)
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
    use crate::token_client::client::{create_token_client, ReqwestTokenClient};
    use mockito::Server;
    use reqwest::Client;
    use serde_json::json;

    fn create_test_client(base_url: String) -> ReqwestTokenClient {
        let config = TokenClientConfig {
            token_endpoint: format!("{}/api/v1/token", base_url).into(),
            token_exchange_endpoint: Some(format!("{}/api/v1/token/exchange", base_url).into()),
        };
        create_token_client(config, Client::new())
    }

    #[tokio::test]
    async fn test_exchange_token_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/token/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "et-exchanged-token",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let result = client
            .exchange_token(
                "bruker-token".to_string(),
                "dev-gcp:some-team:some-app".to_string(),
            )
            .await;

        assert!(result.is_ok());
        let token = result.unwrap();
        assert_eq!(token.access_token, "et-exchanged-token");
        assert_eq!(token.expires_in, 3600);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_exchange_token_feil_gir_token_error() {
        let mut server = Server::new_async().await;

        let forventet_error = json!({
            "error": "invalid_grant",
            "error_description": "The user token is invalid"
        });

        let mock = server
            .mock("POST", "/api/v1/token/exchange")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(forventet_error.to_string())
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let error = client
            .exchange_token(
                "ugyldig-bruker-token".to_string(),
                "dev-gcp:some-team:some-app".to_string(),
            )
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

    #[tokio::test]
    async fn test_exchange_token_ikke_konfigurert() {
        let client = ReqwestTokenClient::new(
            "http://texas/api/v1/token".to_string(),
            None,
            Client::new(),
        );

        let error = client
            .exchange_token(
                "bruker-token".to_string(),
                "dev-gcp:some-team:some-app".to_string(),
            )
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Token exchange endpoint is not configured"),
            "{}",
            error
        );
    }
}
