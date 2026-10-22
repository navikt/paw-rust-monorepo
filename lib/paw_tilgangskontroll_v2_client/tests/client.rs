use mockito::{Mock, Server, ServerGuard};
use paw_tilgangskontroll_v2_api_mock::{
    default_enhet_tilgang_mock_responses, default_person_tilgang_mock_responses,
    init_paw_tilgangskontroll_v2_api_mocks,
};
use paw_tilgangskontroll_v2_client::client::PawTilgangskontrollV2Client;
use paw_tilgangskontroll_v2_client::model::{TilgangResponse, TilgangType};
use std::sync::Arc;
use token_client_stub::TokenClientStub;
use tokio::sync::OnceCell;
use types::identitetsnummer::Identitetsnummer;
use types::nav_enhet_id::NavEnhetId;
use types::nav_ident::NavIdent;

#[tokio::test]
async fn test_tilganger() {
    let context = init().await;

    // Person
    let response_1 = context
        .lese_tilgang_til_person("Z123456", "01017012345")
        .await;
    let response_2 = context
        .skrive_tilgang_til_person("Z123456", "01017012345")
        .await;
    let response_3 = context
        .lese_skrive_tilgang_til_person("Z123456", "01017012345")
        .await;
    let response_4 = context
        .lese_tilgang_til_person("Z123456", "02017012345")
        .await;
    let response_5 = context
        .skrive_tilgang_til_person("Z123456", "02017012345")
        .await;
    let response_6 = context
        .lese_skrive_tilgang_til_person("Z123456", "02017012345")
        .await;
    let response_7 = context
        .lese_tilgang_til_person("Z234567", "01017012345")
        .await;
    let response_8 = context
        .skrive_tilgang_til_person("Z234567", "01017012345")
        .await;
    let response_9 = context
        .lese_skrive_tilgang_til_person("Z234567", "01017012345")
        .await;

    // Enhet
    let response_10 = context.tilgang_til_enhet("Z123456", "1234").await;
    let response_11 = context.tilgang_til_enhet("Z123456", "2345").await;
    let response_12 = context.tilgang_til_enhet("Z234567", "1234").await;
    let response_13 = context.tilgang_til_enhet("Z234567", "2345").await;

    assert_eq!(response_1.har_tilgang, true);
    assert_eq!(response_2.har_tilgang, true);
    assert_eq!(response_3.har_tilgang, true);
    assert_eq!(response_4.har_tilgang, true);
    assert_eq!(response_5.har_tilgang, false);
    assert_eq!(response_6.har_tilgang, false);
    assert_eq!(response_7.har_tilgang, false);
    assert_eq!(response_8.har_tilgang, false);
    assert_eq!(response_9.har_tilgang, false);
    assert_eq!(response_10.har_tilgang, true);
    assert_eq!(response_11.har_tilgang, false);
    assert_eq!(response_12.har_tilgang, false);
    assert_eq!(response_13.har_tilgang, true);
}

struct TestContext {
    #[allow(unused)]
    mockito_server: ServerGuard,
    #[allow(unused)]
    mocks: Vec<Mock>,
    client: PawTilgangskontrollV2Client,
}

impl TestContext {
    async fn lese_tilgang_til_person(
        &self,
        nav_ident: &str,
        identitetsnummer: &str,
    ) -> TilgangResponse {
        self.tilgang_til_person(TilgangType::Lese, nav_ident, identitetsnummer)
            .await
    }
    async fn skrive_tilgang_til_person(
        &self,
        nav_ident: &str,
        identitetsnummer: &str,
    ) -> TilgangResponse {
        self.tilgang_til_person(TilgangType::Skrive, nav_ident, identitetsnummer)
            .await
    }
    async fn lese_skrive_tilgang_til_person(
        &self,
        nav_ident: &str,
        identitetsnummer: &str,
    ) -> TilgangResponse {
        self.tilgang_til_person(TilgangType::LeseSkrive, nav_ident, identitetsnummer)
            .await
    }
    async fn tilgang_til_person(
        &self,
        tilgang: TilgangType,
        nav_ident: &str,
        identitetsnummer: &str,
    ) -> TilgangResponse {
        self.client
            .nav_ansatt_tilgang_til_person(
                tilgang,
                NavIdent::new(nav_ident.to_string()),
                Identitetsnummer::new(identitetsnummer.to_string()).unwrap(),
            )
            .await
            .expect("Kunne ikke hente response")
    }
    async fn tilgang_til_enhet(&self, nav_ident: &str, nav_enhet_id: &str) -> TilgangResponse {
        self.client
            .nav_ansatt_tilgang_til_enhet(
                NavIdent::new(nav_ident.to_string()),
                NavEnhetId::new(nav_enhet_id.to_string()),
            )
            .await
            .expect("Kunne ikke hente response")
    }
}

static INIT: OnceCell<TestContext> = OnceCell::const_new();

async fn init() -> &'static TestContext {
    INIT.get_or_init(|| async {
        let person_tilgang_mock_responses = default_person_tilgang_mock_responses();
        let enhet_tilgang_mock_responses = default_enhet_tilgang_mock_responses();
        let mut mockito_server = Server::new_async().await;
        let tilgangskontroll_api_mock_guard = init_paw_tilgangskontroll_v2_api_mocks(
            &mut mockito_server,
            person_tilgang_mock_responses,
            enhet_tilgang_mock_responses,
        )
        .await
        .expect("Kunne ikke initialisere mock");
        let client = PawTilgangskontrollV2Client::new(
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
