use crate::config::PoaoTilgangClientConfig;
use crate::error::PoaoTilgangClientError;
use crate::model::{EvaluatePoliciesRequest, EvaluatePoliciesResponse};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;

#[derive(Clone)]
pub struct PoaoTilgangClient {
    url: String,
    scope: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl PoaoTilgangClient {
    pub fn from_config(
        config: PoaoTilgangClientConfig,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> Self {
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
    ) -> Self {
        Self {
            url,
            scope,
            http_client,
            token_client,
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn evaluate(
        &self,
        request: EvaluatePoliciesRequest,
    ) -> anyhow::Result<EvaluatePoliciesResponse> {
        let url = format!("{}/api/v1/policy/evaluate", self.url);
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
            reqwest::StatusCode::UNAUTHORIZED => Err(PoaoTilgangClientError::NotAuthorized.into()),
            reqwest::StatusCode::FORBIDDEN => {
                Err(PoaoTilgangClientError::AuthenticationFailed.into())
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                let error = format!("Kall feilet med status {}: {}", status, text);
                Err(PoaoTilgangClientError::UnknownError(error).into())
            }
        }
    }
}
