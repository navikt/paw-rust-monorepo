use crate::client::oppgave_dto::OppgaveDto;
use crate::client::opprett_oppgave_request::OpprettOppgaveRequest;
use crate::config::OppgaveClientConfig;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use texas_client::token_client::M2MTokenClient;

#[derive(Clone)]
pub struct OppgaveApiClient {
    client: Client,
    config: OppgaveClientConfig,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

pub const OPPGAVER_PATH: &str = "/api/v1/oppgaver";

impl OppgaveApiClient {
    pub fn new(
        config: OppgaveClientConfig,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Kunne ikke opprette HTTP-klient");
        Self {
            client,
            config,
            token_client,
        }
    }

    pub async fn opprett_oppgave(
        &self,
        request: &OpprettOppgaveRequest,
    ) -> Result<OppgaveDto, OppgaveApiError> {
        let url = format!("{}{}", self.config.base_url, OPPGAVER_PATH);
        let token = self.hent_token().await?;
        let response = self
            .client
            .post(&url)
            .header("X-Correlation-ID", uuid::Uuid::new_v4().to_string())
            .bearer_auth(token)
            .json(request)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::CREATED => Ok(response.json().await?),
            status => Err(OppgaveApiError::ApiError {
                status,
                message: response.text().await.unwrap_or_default(),
            }),
        }
    }

    async fn hent_token(&self) -> Result<String> {
        let token_response = self
            .token_client
            .get_token(self.config.scope.to_string())
            .await?;
        Ok(token_response.access_token)
    }
}

#[derive(Debug, thiserror::Error, strum::EnumDiscriminants)]
#[strum_discriminants(derive(strum::EnumIter, strum::Display))]
pub enum OppgaveApiError {
    #[error("HTTP-feil: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("API-feil ({status}): {message}")]
    ApiError {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("Token-feil: {0}")]
    TokenError(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::opprett_oppgave_request::PrioritetV1;
    use crate::test_utils::{MockTokenClient, test_client_config};
    use anyhow::Result;
    use chrono::Utc;
    use mockito::Server;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_opprett_oppgave_vellykket() {
        let mut server = Server::new_async().await;

        let oppgave_id = 12345;
        let oppgave_mock_api = server
            .mock("POST", OPPGAVER_PATH)
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": oppgave_id,
                    "tildeltEnhetsnr": "4863",
                    "oppgavetype": "JFR",
                    "tema": "KON",
                    "prioritet": "NORM",
                    "status": "OPPRETTET",
                    "aktivDato": "2026-02-16",
                    "versjon": 1,
                })
                .to_string(),
            )
            .create_async()
            .await;

        let token_client = Arc::new(MockTokenClient);
        let config = test_client_config(server.url());
        let client = OppgaveApiClient::new(config, token_client);

        let request = OpprettOppgaveRequest {
            personident: Some("12345678910".to_string()),
            aktiv_dato: Utc::now().format("%Y-%m-%d").to_string(),
            oppgavetype: "KONT_BRUK".to_string(),
            prioritet: PrioritetV1::Norm,
            tema: "GEN".to_string(),
            ..Default::default()
        };

        let oppgave = client.opprett_oppgave(&request).await.unwrap();
        assert_eq!(oppgave.id, oppgave_id);
        assert_eq!(oppgave.tildelt_enhetsnr, "4863");

        oppgave_mock_api.assert_async().await;
    }
}