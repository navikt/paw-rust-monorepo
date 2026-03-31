use std::sync::Arc;

use crate::config::TokenClientConfig;
use crate::error::TexasClientError;
use crate::response::TokenResponse;
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[derive(Clone)]
pub struct ReqwestTokenClient {
    pub(super) inner: Arc<ReqwestTokenClientRef>,
}

pub(super) struct ReqwestTokenClientRef {
    pub token_endpoint: String,
    pub token_exchange_endpoint: Option<String>,
    pub client: Client,
}

pub fn create_token_client(config: TokenClientConfig, client: Client) -> ReqwestTokenClient {
    ReqwestTokenClient::new(
        config.token_endpoint.into_inner(),
        config.token_exchange_endpoint.map(|e| e.into_inner()),
        client,
    )
}

impl ReqwestTokenClient {
    pub(super) fn new(
        token_endpoint: String,
        token_exchange_endpoint: Option<String>,
        client: Client,
    ) -> Self {
        Self {
            inner: Arc::new(ReqwestTokenClientRef {
                token_endpoint,
                token_exchange_endpoint,
                client,
            }),
        }
    }
}

pub(super) async fn parse_error_response(
    response: reqwest::Response,
    target: String,
) -> anyhow::Error {
    let status = response.status().as_u16();
    let error_response = response
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
pub(super) struct TexasErrorResponse {
    pub error: String,
    pub error_description: String,
}

pub(super) fn token_response_parse_error(target: String) -> anyhow::Error {
    TexasClientError::Response {
        status: 200,
        target,
    }
    .into()
}

pub(super) fn request_send_error(e: &reqwest::Error, target: String) -> TexasClientError {
    TexasClientError::Request {
        status: e.status().map(|s| s.as_u16()).unwrap_or(0),
        target,
    }
}

pub(super) async fn parse_token_response(
    response: reqwest::Response,
    target: String,
) -> Result<TokenResponse> {
    match response.status().is_success() {
        false => Err(parse_error_response(response, target).await),
        true => response
            .json::<TokenResponse>()
            .await
            .map_err(|_| token_response_parse_error(target)),
    }
}
