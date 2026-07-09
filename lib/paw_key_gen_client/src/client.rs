use crate::config::PawKeyGenClientConfig;
use crate::error::PawKeyGenClientError;
use crate::model::{IdentitetRequest, IdentitetResponse, KeyRequest, KeyResponse};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;

#[derive(Clone)]
pub struct PawKeyGenClient {
    url: String,
    scope: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl PawKeyGenClient {
    pub fn from_config(
        config: PawKeyGenClientConfig,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> PawKeyGenClient {
        Self::new(
            config.url.into_inner(),
            config.target_scope.into_inner(),
            http_client,
            token_client,
        )
    }

    pub fn new(
        url: String,
        scope: String,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> PawKeyGenClient {
        PawKeyGenClient {
            url,
            scope,
            http_client,
            token_client,
        }
    }

    pub async fn hent(&self, identitet: String) -> anyhow::Result<KeyResponse> {
        let url = format!("{}/api/v2/hent", self.url);
        let request = KeyRequest { ident: identitet };
        self.post(url, request).await
    }

    pub async fn finn_identiteter(&self, identitet: String) -> anyhow::Result<IdentitetResponse> {
        let url = format!("{}/api/v2/identiteter", self.url);
        let request = IdentitetRequest { identitet };
        self.post(url, request).await
    }

    async fn post<S: Serialize, T: DeserializeOwned>(
        &self,
        url: String,
        request: S,
    ) -> anyhow::Result<T> {
        let token = match self.token_client.get_token(self.scope.clone()).await {
            Ok(token) => token,
            Err(e) => return Err(e),
        };
        let response = self
            .http_client
            .post(url)
            .json(&request)
            .bearer_auth(token.access_token)
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(response.json().await?),
            reqwest::StatusCode::UNAUTHORIZED => Err(PawKeyGenClientError::NotAuthorized.into()),
            reqwest::StatusCode::FORBIDDEN => {
                Err(PawKeyGenClientError::AuthenticationFailed.into())
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                let error = format!("Kall feilet med status {}: {}", status, text);
                Err(PawKeyGenClientError::UnknownError(error).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::PawKeyGenClient;
    use crate::model::IdentitetType;
    use mockito::Server;
    use serde_env_field::EnvField;
    use serde_json::json;
    use std::str::FromStr;
    use std::sync::Arc;
    use texas_client::config::TokenClientConfig;
    use texas_client::token_client::create_token_client;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_finn_identiteter() {
        let mut mockito_server = Server::new_async().await;
        let _idenititeter_endpoint_mock = mockito_server
            .mock("POST", "/api/v2/identiteter")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "recordKey": -1337,
                    "arbeidssoekerId": 1337,
                    "identiteter": [
                        {
                            "identitet": "01017012345",
                            "type": "FOLKEREGISTERIDENT",
                            "gjeldende": true
                        }
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;
        let _token_endpoint_mock = mockito_server
            .mock("POST", "/api/v1/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "test-token",
                    "expires_in": 1337,
                    "token_type": "Bearer"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let token_url = format!("{}/api/v1/token", mockito_server.url());

        let http_client = reqwest::Client::new();
        let token_client_config = TokenClientConfig {
            token_endpoint: EnvField::from_str(token_url.as_str()).unwrap(),
            token_exchange_endpoint: None,
        };
        let token_client = Arc::new(create_token_client(
            token_client_config,
            http_client.clone(),
        ));
        let client = PawKeyGenClient::new(
            mockito_server.url(),
            "test-scope".to_string(),
            http_client.clone(),
            token_client,
        );

        let response = client
            .finn_identiteter("01017012345".to_string())
            .await
            .unwrap();

        assert_eq!(response.record_key, Some(-1337));
        assert_eq!(response.arbeidssoeker_id, Some(1337));
        assert_eq!(response.identiteter.len(), 1);
        let identitet = response.identiteter.get(0).unwrap();
        assert_eq!(identitet.identitet, "01017012345");
        assert_eq!(identitet.identitet_type, IdentitetType::Folkeregisterident);
        assert_eq!(identitet.gjeldende, true);
        assert!(response.pdl_identiteter.is_none());
        assert!(response.konflikter.is_none());
    }
}
