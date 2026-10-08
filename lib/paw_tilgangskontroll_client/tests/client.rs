use mockito::{Mock, Server, ServerGuard};
use paw_tilgangskontroll_api_mock::{
    default_tilgang_mock_responses, init_paw_tilgangskontroll_api_mocks,
};
use paw_tilgangskontroll_client::client::PawTilgangskontrollClient;
use paw_tilgangskontroll_client::model::{TilgangResponse, TilgangType};
use std::sync::Arc;
use token_client_stub::TokenClientStub;
use tokio::sync::OnceCell;

#[tokio::test]
async fn test_finn_identiteter() {
    let context = init().await;

    let response_1 = context.lese_tilgang("01017012345", "2012345").await;
    let response_2 = context.lese_tilgang("02017012345", "2012345").await;
    let response_3 = context.lese_tilgang("01017012345", "2112345").await;
    let response_4 = context.lese_tilgang("02017012345", "2112345").await;

    assert!(response_1.har_tilgang);
    assert!(!response_2.har_tilgang);
    assert!(!response_3.har_tilgang);
    assert!(response_4.har_tilgang);
}

struct TestContext {
    #[allow(unused)]
    mockito_server: ServerGuard,
    #[allow(unused)]
    mocks: Vec<Mock>,
    client: PawTilgangskontrollClient,
}

impl TestContext {
    async fn lese_tilgang(&self, identitetsnummer: &str, nav_ansatt_id: &str) -> TilgangResponse {
        self.client
            .tilgang(
                identitetsnummer.to_string(),
                nav_ansatt_id.to_string(),
                TilgangType::Lese,
            )
            .await
            .expect("Kunne ikke hente response")
    }
}

static INIT: OnceCell<TestContext> = OnceCell::const_new();

async fn init() -> &'static TestContext {
    INIT.get_or_init(|| async {
        let mock_responses = default_tilgang_mock_responses();
        let mut mockito_server = Server::new_async().await;
        let tilgangskontroll_api_mock_guard =
            init_paw_tilgangskontroll_api_mocks(&mut mockito_server, mock_responses)
                .await
                .expect("Kunne ikke initialisere mock");
        let client = PawTilgangskontrollClient::new(
            mockito_server.url(),
            "test-scope".to_string(),
            reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("Failed to build reqwest client"),
            Arc::new(TokenClientStub::new()),
        );

        TestContext {
            mockito_server,
            mocks: tilgangskontroll_api_mock_guard.mocks,
            client,
        }
    })
    .await
}
