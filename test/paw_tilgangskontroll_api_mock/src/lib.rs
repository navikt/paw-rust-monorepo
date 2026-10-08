use mockito::{Matcher, Mock, ServerGuard};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;

#[derive(PartialEq, Eq, Hash)]
pub struct TilgangMockRequest {
    pub identitetsnummer: String,
    pub nav_ansatt_id: String,
}

impl TilgangMockRequest {
    pub fn new(identitetsnummer: &str, nav_ansatt_id: &str) -> Self {
        Self {
            identitetsnummer: identitetsnummer.to_string(),
            nav_ansatt_id: nav_ansatt_id.to_string(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TilgangMockResponse {
    pub har_tilgang: bool,
}

impl TilgangMockResponse {
    pub fn new(har_tilgang: bool) -> Self {
        Self { har_tilgang }
    }
}

pub struct PawTilgangskontrollMockGuard {
    pub mocks: Vec<Mock>,
}

pub fn default_tilgang_mock_responses() -> HashMap<TilgangMockRequest, TilgangMockResponse> {
    let mut map = HashMap::new();
    map.insert(
        TilgangMockRequest::new("01017012345", "2012345"),
        TilgangMockResponse::new(true),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "2012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("01017012345", "2112345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "2112345"),
        TilgangMockResponse::new(true),
    );
    map
}

pub async fn init_paw_tilgangskontroll_api_mocks(
    mockito_server: &mut ServerGuard,
    mock_responses: HashMap<TilgangMockRequest, TilgangMockResponse>,
) -> Result<PawTilgangskontrollMockGuard, Box<dyn Error>> {
    let _ = env_logger::try_init();
    let mut mocks = vec![];
    for (mock_request, mock_response) in &mock_responses {
        mocks
            .push(tilgang_api_mock(mockito_server, mock_request, mock_response).await);
    }

    Ok(PawTilgangskontrollMockGuard { mocks })
}

async fn tilgang_api_mock(
    mockito_server: &mut ServerGuard,
    mock_request: &TilgangMockRequest,
    mock_response: &TilgangMockResponse,
) -> Mock {
    let identitetsnummer = &mock_request.identitetsnummer;
    let nav_ansatt_id = &mock_request.nav_ansatt_id;
    mockito_server
        .mock("POST", "/api/v1/tilgang")
        .match_body(Matcher::PartialJson(json!({
            "identitetsnummer": identitetsnummer,
            "navAnsattId": nav_ansatt_id
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(mock_response).expect("Failed to serialize mock response"))
        .create_async()
        .await
}
