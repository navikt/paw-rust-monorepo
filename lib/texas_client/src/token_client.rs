use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use crate::{
    error::TexasClientError, request::{get_nais_token_endpoint, M2MTokenRequest},
    M2MTokenClient,
    TokenResponse,
};

#[derive(Clone)]
pub struct ReqwestTokenClient {
    inner: Arc<ReqwestTokenClientRef>,
}

pub struct ReqwestTokenClientRef {
    token_endpoint: String,
    client: reqwest::Client,
}

pub fn create_token_client(client: Client) -> Result<ReqwestTokenClient> {
    ReqwestTokenClient::new(client)
}

impl ReqwestTokenClient {
    fn new(client: reqwest::Client) -> Result<Self> {
        let token_endpoint = get_nais_token_endpoint()?;
        Ok(Self {
            inner: Arc::new(ReqwestTokenClientRef {
                token_endpoint,
                client,
            }),
        })
    }
    fn new_with_endpoint(token_endpoint: String, client: reqwest::Client) -> Self {
        Self {
            inner: Arc::new(ReqwestTokenClientRef {
                token_endpoint,
                client,
            }),
        }
    }
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
