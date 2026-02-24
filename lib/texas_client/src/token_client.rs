use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use crate::config::TokenClientConfig;
use crate::response::TokenResponse;
use crate::{
    error::TexasClientError,
    request::M2MTokenRequest,
};

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

        let token_response =
            response
                .json::<TokenResponse>()
                .await
                .map_err(|e| TexasClientError::Response {
                    status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                    target: target.clone(),
                })?;
        Ok(token_response)
    }
}
