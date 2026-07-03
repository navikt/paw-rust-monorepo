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
