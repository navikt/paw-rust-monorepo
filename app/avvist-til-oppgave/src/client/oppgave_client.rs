use crate::client::oppgave_dto::OppgaveDto;
use crate::client::opprett_oppgave_request::OpprettOppgaveRequest;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use texas_client::M2MTokenClient;

#[derive(Clone)]
pub struct OppgaveApiClient {
    client: Client,
    base_url: String,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl OppgaveApiClient {
    pub fn new(base_url: String, token_client: Arc<dyn M2MTokenClient + Send + Sync>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Kunne ikke opprette HTTP-klient");
        Self {
            client,
            base_url,
            token_client,
        }
    }

    pub async fn opprett_oppgave(&self, request: &OpprettOppgaveRequest) -> Result<OppgaveDto> {
        let url = format!("{}/api/v1/oppgaver", self.base_url);
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
            }
            .into()),
        }
    }

    pub async fn hent_oppgave(&self, oppgave_id: i64) -> Result<OppgaveDto> {
        let url = format!("{}/api/v1/oppgaver/{}", self.base_url, oppgave_id);
        let token = self.hent_token().await?;
        let response = self
            .client
            .get(&url)
            .header("X-Correlation-ID", uuid::Uuid::new_v4().to_string())
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => Ok(response.json().await?),
            reqwest::StatusCode::NOT_FOUND => Err(OppgaveApiError::ApiError {
                status: reqwest::StatusCode::NOT_FOUND,
                message: "Oppgave ikke funnet".to_string(),
            }
            .into()),
            status => Err(OppgaveApiError::ApiError {
                status,
                message: response.text().await.unwrap_or_default(),
            }
            .into()),
        }
    }

    async fn hent_token(&self) -> Result<String> {
        let target = format!("{}/token", self.base_url);
        let token_response = self.token_client.get_token(target).await?;
        Ok(token_response.access_token)
    }
}

#[derive(Debug, thiserror::Error)]
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
    use anyhow::Result;
    use async_trait::async_trait;
    use mockito::Server;
    use serde_json::json;
    use std::sync::Arc;
    use texas_client::TokenResponse;

    struct MockTokenClient;
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

    #[tokio::test]
    async fn test_opprett_oppgave_vellykket() {
        let mut server = Server::new_async().await;

        let oppgave_id = 12345;
        let oppgave_mock_api = server
            .mock("POST", "/api/v1/oppgaver")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": oppgave_id,
                    "tildelt_enhetsnr": "4863",
                    "oppgavetype": "JFR",
                    "tema": "KON",
                    "prioritet": "NORM",
                    "status": "OPPRETTET",
                    "aktiv_dato": "2026-02-16",
                    "versjon": 1,
                })
                .to_string(),
            )
            .create_async()
            .await;

        let token_client = Arc::new(MockTokenClient);
        let client = OppgaveApiClient::new(server.url(), token_client);

        let request = OpprettOppgaveRequest {
            personident: Some("12345678901".to_string()),
            tildelt_enhetsnr: "4863".to_string(),
            oppgavetype: "JFR".to_string(),
            tema: "KON".to_string(),
            prioritet: PrioritetV1::Norm,
            aktiv_dato: "2026-02-16".to_string(),
            ..Default::default()
        };

        let oppgave = client.opprett_oppgave(&request).await.unwrap();
        assert_eq!(oppgave.id, oppgave_id);
        assert_eq!(oppgave.tildelt_enhetsnr, "4863");

        oppgave_mock_api.assert_async().await;
    }

    #[tokio::test]
    async fn test_hent_oppgave_vellykket() {
        let mut server = Server::new_async().await;

        let oppgave_id = 12345;
        let oppgave_mock_api = server
            .mock("GET", format!("/api/v1/oppgaver/{}", oppgave_id).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": oppgave_id,
                    "tildelt_enhetsnr": "4863",
                    "oppgavetype": "JFR",
                    "tema": "KON",
                    "prioritet": "NORM",
                    "status": "OPPRETTET",
                    "aktiv_dato": "2026-02-16",
                    "versjon": 1,
                })
                .to_string(),
            )
            .create_async()
            .await;

        let token_client = Arc::new(MockTokenClient);
        let client = OppgaveApiClient::new(server.url(), token_client);

        let oppgave = client.hent_oppgave(oppgave_id).await.unwrap();

        assert_eq!(oppgave.id, oppgave_id);
        assert_eq!(oppgave.tildelt_enhetsnr, "4863");
        assert!(matches!(oppgave.prioritet, PrioritetV1::Norm));

        oppgave_mock_api.assert_async().await;
    }
}
