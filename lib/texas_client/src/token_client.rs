use std::sync::Arc;

use crate::config::TokenClientConfig;
use crate::response::TokenResponse;
use crate::{error::TexasClientError, request::M2MTokenRequest};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

#[derive(Clone)]
pub struct ReqwestTokenClient {
    inner: Arc<ReqwestTokenClientRef>,
}

pub struct ReqwestTokenClientRef {
    token_endpoint: String,
    client: Client,
}

pub fn create_token_client(
    token_endpoint: TokenClientConfig,
    client: Client,
) -> ReqwestTokenClient {
    ReqwestTokenClient::new_with_endpoint(token_endpoint.token_endpoint, client)
}

impl ReqwestTokenClient {
    fn new_with_endpoint(token_endpoint: String, client: Client) -> Self {
        Self {
            inner: Arc::new(ReqwestTokenClientRef {
                token_endpoint,
                client,
            }),
        }
    }
}

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
            .map_err(|e| TexasClientError::Request {
                status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                target: target.clone(),
            })?;

        match response.status().is_success() {
            false => Err(parse_error_response(response, target).await),
            true => response.json::<TokenResponse>().await.map_err(|_| {
                TexasClientError::Response {
                    status: 200,
                    target,
                }
                .into()
            }),
        }
    }
}

async fn parse_error_response(response: reqwest::Response, target: String) -> anyhow::Error {
    let status = response.status().as_u16();
    let error_response =
        response
            .json::<TexasErrorResponse>()
            .await
            .unwrap_or(TexasErrorResponse {
                error: "unknown".to_string(),
                error_description: "Kunne ikke parse feilrespons fra Texas".to_string(),
            });

    TexasClientError::TokenError {
        status,
        target,
        error: error_response.error,
        error_description: error_response.error_description,
    }
    .into()
}

#[derive(Debug, Deserialize)]
struct TexasErrorResponse {
    error: String,
    error_description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    fn create_test_client(base_url: String) -> ReqwestTokenClient {
        let client = Client::new();
        let config = TokenClientConfig {
            token_endpoint: format!("{}/api/v1/token", base_url),
        };
        create_token_client(config, client)
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
