use crate::config::OppgaveClientConfig;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
use texas_client::response::TokenResponse;
use texas_client::token_client::M2MTokenClient;

pub(crate) struct MockTokenClient;

#[async_trait]
impl M2MTokenClient for MockTokenClient {
    async fn get_token(&self, _target: String) -> Result<TokenResponse> {
        Ok(TokenResponse {
            access_token: "dummy-token".to_string(),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }
}

pub(crate) fn lag_kafka_melding(offset: i64, json: &str) -> OwnedMessage {
    OwnedMessage::new(
        Some(json.as_bytes().to_vec()),
        None,
        "test-topic".to_string(),
        Timestamp::CreateTime(Utc::now().timestamp_micros()),
        0,
        offset,
        Some(OwnedHeaders::new()),
    )
}

pub(crate) fn test_client_config(base_url: String) -> OppgaveClientConfig {
    OppgaveClientConfig {
        base_url: base_url.into(),
        scope: "test-scope".to_string().into(),
    }
}
