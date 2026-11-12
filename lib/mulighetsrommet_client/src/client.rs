use crate::config::MulighetsrommetClientConfig;
use crate::error::MulighetsrommetClientError;
use mulighetsrommet::tiltakshistorikk::{TiltakshistorikkV1Request, TiltakshistorikkV1Response};
use paw_observability::client::trace_context_aware_headers;
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;

#[derive(Clone)]
pub struct MulighetsrommetClient {
    inner: Arc<Inner>,
}

struct Inner {
    url: String,
    scope: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl MulighetsrommetClient {
    pub fn from_config(
        config: MulighetsrommetClientConfig,
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
            inner: Arc::new(Inner {
                url: format!("{}/api/v1/historikk", url.trim_end_matches('/')),
                scope,
                http_client,
                token_client,
            }),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn hent_historikk(
        &self,
        identer: &Vec<String>,
    ) -> Result<TiltakshistorikkV1Response, MulighetsrommetClientError> {
        let token = self
            .inner
            .token_client
            .get_token(self.inner.scope.clone())
            .await
            .map_err(MulighetsrommetClientError::Token)?;
        let request = TiltakshistorikkV1Request {
            identer: identer.to_vec(),
        };
        let response = self
            .inner
            .http_client
            .post(&self.inner.url)
            .headers(trace_context_aware_headers())
            .bearer_auth(token.access_token)
            .json(&request)
            .send()
            .await
            .map_err(|error| MulighetsrommetClientError::Request(error.without_url()))?;

        if response.status() != reqwest::StatusCode::OK {
            return Err(MulighetsrommetClientError::HttpStatus(response.status()));
        }

        response
            .json()
            .await
            .map_err(|error| MulighetsrommetClientError::Response(error.without_url()))
    }
}
