use std::{error::Error, fmt::Display, sync::Arc};

use paw_rust_base::{
    env_var::{EnvVarNotFoundError, get_env},
    error_handling::AppError,
};
use reqwest::Client;

use crate::{
    M2MTokenClient, TokenResponse,
    request::{M2MTokenRequest, get_nais_token_endpoint},
    texas_error::TexasClientError,
};

#[derive(Clone)]
pub struct ReqwestTokenClient {
    inner: Arc<ReqwestTokenClientRef>,
}

pub struct ReqwestTokenClientRef {
    token_endpoint: String,
    client: reqwest::Client,
}

pub fn create_token_client(client: Client) -> Result<ReqwestTokenClient, Box<dyn AppError>> {
    ReqwestTokenClient::new(client)
}

impl ReqwestTokenClient {
    fn new(client: reqwest::Client) -> Result<Self, Box<dyn AppError>> {
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

impl M2MTokenClient for ReqwestTokenClient {
    async fn get_token(&self, target: String) -> Result<TokenResponse, Box<dyn AppError>> {
        let request = M2MTokenRequest::new(target.clone());
        let response = self
            .inner
            .client
            .post(&self.inner.token_endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|err| TexasClientError {
                texas_response_code: err.status().map(|s| s.as_u16()).unwrap_or(0),
                target: target.clone(),
                message: format!("Failed to send request: {}", err),
            })?;

        let token_response =
            response
                .json::<TokenResponse>()
                .await
                .map_err(|err| TexasClientError {
                    texas_response_code: 0,
                    target: target.clone(),
                    message: format!("Failed to parse response: {}", err),
                })?;
        Ok(token_response)
    }
}
