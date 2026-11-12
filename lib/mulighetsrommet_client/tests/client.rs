use async_trait::async_trait;
use mockito::{Matcher, Server};
use mulighetsrommet::vo::{Deltakelse, TeamTiltakStatus};
use mulighetsrommet_client::client::MulighetsrommetClient;
use mulighetsrommet_client::config::MulighetsrommetClientConfig;
use mulighetsrommet_client::error::MulighetsrommetClientError;
use reqwest::StatusCode;
use std::sync::Arc;
use texas_client::response::TokenResponse;
use texas_client::token_client::M2MTokenClient;
use token_client_stub::TokenClientStub;

fn client(url: String) -> MulighetsrommetClient {
    MulighetsrommetClient::from_config(
        MulighetsrommetClientConfig {
            url: url.into(),
            target_scope: "test-scope".to_string().into(),
        },
        reqwest::Client::builder().no_proxy().build().unwrap(),
        Arc::new(TokenClientStub::new()),
    )
}

#[tokio::test]
async fn sends_identities_and_bearer_token_and_reads_history() {
    let mut server = Server::new_async().await;
    let token = TokenClientStub::new()
        .get_token("test-scope".to_string())
        .await
        .unwrap();
    let mock = server
        .mock("POST", "/api/v1/historikk")
        .match_header(
            "authorization",
            format!("Bearer {}", token.access_token).as_str(),
        )
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(serde_json::json!({
            "identer": ["01017012345", "41017012345"]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            serde_json::json!({
                "historikk": [{
                    "type": "TeamTiltakAvtale",
                    "norskIdent": "01017012345",
                    "opprettetTidspunkt": "2026-01-01T00:00:00Z",
                    "oppdatertTidspunkt": "2026-01-02T00:00:00Z",
                    "id": "avtale-id",
                    "tittel": "Arbeidstrening",
                    "tiltakstype": {"tiltakskode": "ARBEIDSTRENING", "navn": "Arbeidstrening"},
                    "status": "GJENNOMFORES",
                    "arbeidsgiver": {"organisasjonsnummer": "123456789"}
                }]
            })
            .to_string(),
        )
        .create_async()
        .await;

    let response = client(format!("{}/", server.url()))
        .hent_historikk(&vec!["01017012345".to_string(), "41017012345".to_string()])
        .await
        .unwrap();

    assert_eq!(response.historikk.len(), 1);
    let Deltakelse::TeamTiltakAvtale(avtale) = &response.historikk[0] else {
        panic!("Forventet TeamTiltakAvtale");
    };
    assert_eq!(avtale.id, "avtale-id");
    assert_eq!(avtale.status, TeamTiltakStatus::Gjennomfores);
    mock.assert_async().await;
}

#[tokio::test]
async fn reads_empty_history() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v1/historikk")
        .with_status(200)
        .with_body(r#"{"historikk":[]}"#)
        .create_async()
        .await;

    let response = client(server.url())
        .hent_historikk(&vec!["01017012345".to_string()])
        .await
        .unwrap();

    assert!(response.historikk.is_empty());
    mock.assert_async().await;
}

#[tokio::test]
async fn preserves_http_error_status_without_response_body() {
    let mut server = Server::new_async().await;
    for status in [
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        let mock = server
            .mock("POST", "/api/v1/historikk")
            .with_status(usize::from(status.as_u16()))
            .with_body("sensitive-response-body")
            .create_async()
            .await;

        let error = client(server.url())
            .hent_historikk(&vec!["01017012345".to_string()])
            .await
            .unwrap_err();

        assert!(
            matches!(error, MulighetsrommetClientError::HttpStatus(actual) if actual == status)
        );
        assert!(!format!("{error:?}").contains("sensitive-response-body"));
        mock.assert_async().await;
    }
}

#[tokio::test]
async fn rejects_invalid_response() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v1/historikk")
        .with_status(200)
        .with_body(r#"{"unexpected":[]}"#)
        .create_async()
        .await;

    let error = client(server.url())
        .hent_historikk(&vec!["01017012345".to_string()])
        .await
        .unwrap_err();

    assert!(matches!(error, MulighetsrommetClientError::Response(_)));
    mock.assert_async().await;
}

struct FailingTokenClient;

#[tokio::test]
async fn propagates_request_error() {
    let error = client("not a valid URL".to_string())
        .hent_historikk(&vec!["01017012345".to_string()])
        .await
        .unwrap_err();

    assert!(matches!(error, MulighetsrommetClientError::Request(_)));
}

#[async_trait]
impl M2MTokenClient for FailingTokenClient {
    async fn get_token(&self, target: String) -> anyhow::Result<TokenResponse> {
        assert_eq!(target, "test-scope");
        Err(anyhow::anyhow!("Token unavailable"))
    }
}

#[tokio::test]
async fn propagates_token_error_without_calling_api() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v1/historikk")
        .expect(0)
        .create_async()
        .await;
    let client = MulighetsrommetClient::new(
        server.url(),
        "test-scope".to_string(),
        reqwest::Client::new(),
        Arc::new(FailingTokenClient),
    );

    let error = client
        .hent_historikk(&vec!["01017012345".to_string()])
        .await
        .unwrap_err();

    assert!(matches!(error, MulighetsrommetClientError::Token(_)));
    mock.assert_async().await;
}
