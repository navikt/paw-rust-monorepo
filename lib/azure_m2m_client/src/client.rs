use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::Mutex;

use crate::config::AzureAdM2MConfig;
use crate::error::AzureAdM2MClientError;
use crate::response::{TokenErrorResponse, TokenResponse};

/// Number of seconds before actual expiry at which a cached token is considered stale.
const TOKEN_EXPIRY_BUFFER_SECS: u64 = 30;

#[async_trait]
pub trait M2MTokenClient: Send + Sync {
    async fn get_token(&self, scope: String) -> Result<String, AzureAdM2MClientError>;
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

impl CachedToken {
    fn is_valid(&self) -> bool {
        self.expires_at > Instant::now()
    }
}

struct AzureAdM2MClientInner {
    token_endpoint_url: String,
    client_id: String,
    client_secret: String,
    http_client: Client,
    cache: Mutex<HashMap<String, CachedToken>>,
}

/// OAuth2 client credentials (M2M) token client for Azure AD / Entra ID.
///
/// Fetches tokens using the `client_credentials` grant and caches them until
/// [`TOKEN_EXPIRY_BUFFER_SECS`] before their expiry. Cheap to clone — the
/// underlying state is reference-counted.
#[derive(Clone)]
pub struct AzureAdM2MClient {
    inner: Arc<AzureAdM2MClientInner>,
}

impl AzureAdM2MClient {
    pub fn new(config: AzureAdM2MConfig, http_client: Client) -> Self {
        Self {
            inner: Arc::new(AzureAdM2MClientInner {
                token_endpoint_url: config.token_endpoint_url,
                client_id: config.client_id,
                client_secret: config.client_secret,
                http_client,
                cache: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn from_config(config: AzureAdM2MConfig) -> Self {
        Self::new(config, Client::new())
    }
}

#[async_trait]
impl M2MTokenClient for AzureAdM2MClient {
    async fn get_token(&self, scope: String) -> Result<String, AzureAdM2MClientError> {
        {
            let cache = self.inner.cache.lock().await;
            if let Some(cached) = cache.get(&scope) {
                if cached.is_valid() {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        let token = fetch_token(&self.inner, &scope).await?;

        let expires_at = Instant::now()
            + Duration::from_secs(token.expires_in.saturating_sub(TOKEN_EXPIRY_BUFFER_SECS));

        let mut cache = self.inner.cache.lock().await;
        cache.insert(
            scope,
            CachedToken {
                access_token: token.access_token.clone(),
                expires_at,
            },
        );

        Ok(token.access_token)
    }
}

async fn fetch_token(
    inner: &AzureAdM2MClientInner,
    scope: &str,
) -> Result<TokenResponse, AzureAdM2MClientError> {
    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", inner.client_id.as_str()),
        ("client_secret", inner.client_secret.as_str()),
        ("scope", scope),
    ];

    let response = inner
        .http_client
        .post(&inner.token_endpoint_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AzureAdM2MClientError::Request {
            scope: scope.to_string(),
            source: e,
        })?;

    if response.status().is_success() {
        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| AzureAdM2MClientError::Deserialization {
                scope: scope.to_string(),
                source: e,
            })
    } else {
        let status = response.status().as_u16();
        let error_response = response
            .json::<TokenErrorResponse>()
            .await
            .unwrap_or(TokenErrorResponse {
                error: "unknown".to_string(),
                error_description: "Could not parse error response from token endpoint".to_string(),
            });
        Err(AzureAdM2MClientError::TokenError {
            status,
            scope: scope.to_string(),
            error: error_response.error,
            error_description: error_response.error_description,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    fn create_test_client(base_url: String) -> AzureAdM2MClient {
        let config = AzureAdM2MConfig {
            token_endpoint_url: format!("{}/token", base_url),
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
        };
        AzureAdM2MClient::from_config(config)
    }

    #[tokio::test]
    async fn test_get_token_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "test-access-token",
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
        assert_eq!(result.unwrap(), "test-access-token");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_token_cached() {
        let mut server = Server::new_async().await;
        // Only one HTTP call should be made for two get_token calls with the same scope.
        let mock = server
            .mock("POST", "/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "cached-token",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let scope = "api://test-scope/.default".to_string();
        let first = client.get_token(scope.clone()).await.unwrap();
        let second = client.get_token(scope).await.unwrap();

        assert_eq!(first, "cached-token");
        assert_eq!(second, "cached-token");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_different_scopes_cached_independently() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "some-token",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })
                .to_string(),
            )
            .expect(2)
            .create_async()
            .await;

        let client = create_test_client(server.url());
        client
            .get_token("api://scope-a/.default".to_string())
            .await
            .unwrap();
        client
            .get_token("api://scope-b/.default".to_string())
            .await
            .unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_token_error_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/token")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "error": "invalid_client",
                    "error_description": "The client credentials are invalid"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = create_test_client(server.url());
        let result = client
            .get_token("api://test-scope/.default".to_string())
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid_client"), "{}", err);
        assert!(
            err.contains("The client credentials are invalid"),
            "{}",
            err
        );
        mock.assert_async().await;
    }
}
