use mockito::{Matcher, Mock, ServerGuard};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use strum::{AsRefStr, EnumString};

#[derive(Eq, PartialEq, Hash)]
pub struct TilgangMockRequest {
    pub identitetsnummer: String,
    pub nav_ansatt_id: String,
    pub tilgang: TilgangType,
}

impl TilgangMockRequest {
    pub fn new(identitetsnummer: &str, nav_ansatt_id: &str, tilgang: TilgangType) -> Self {
        Self {
            identitetsnummer: identitetsnummer.to_string(),
            nav_ansatt_id: nav_ansatt_id.to_string(),
            tilgang,
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

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TilgangType {
    Lese,
    Skrive,
    LeseSkrive,
}

pub fn enum_type_not_found(type_: &str) -> EnumTypeParseError {
    EnumTypeParseError::UkjentType(type_.to_string())
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EnumTypeParseError {
    #[error("Ukjent enum: {0}")]
    UkjentType(String),
}

pub struct PawTilgangskontrollMockGuard {
    pub mocks: Vec<Mock>,
}

pub fn default_tilgang_mock_responses() -> HashMap<TilgangMockRequest, TilgangMockResponse> {
    let mut map = HashMap::new();
    map.insert(
        TilgangMockRequest::new("01017012345", "Z123456", TilgangType::Lese),
        TilgangMockResponse::new(true),
    );
    map.insert(
        TilgangMockRequest::new("01017012345", "Z123456", TilgangType::Skrive),
        TilgangMockResponse::new(true),
    );
    map.insert(
        TilgangMockRequest::new("01017012345", "Z123456", TilgangType::LeseSkrive),
        TilgangMockResponse::new(true),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "Z123456", TilgangType::Lese),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "Z123456", TilgangType::Skrive),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "Z123456", TilgangType::LeseSkrive),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("01017012345", "Z234567", TilgangType::Lese),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("01017012345", "Z234567", TilgangType::Skrive),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("01017012345", "Z234567", TilgangType::LeseSkrive),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "Z234567", TilgangType::Lese),
        TilgangMockResponse::new(true),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "Z234567", TilgangType::Skrive),
        TilgangMockResponse::new(false),
    );
    map.insert(
        TilgangMockRequest::new("02017012345", "Z234567", TilgangType::LeseSkrive),
        TilgangMockResponse::new(false),
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
        mocks.push(tilgang_api_mock(mockito_server, mock_request, mock_response).await);
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
    let tilgang = &mock_request.tilgang.as_ref().to_string();
    mockito_server
        .mock("POST", "/api/v1/tilgang")
        .match_body(Matcher::PartialJson(json!({
            "identitetsnummer": identitetsnummer,
            "navAnsattId": nav_ansatt_id,
            "tilgang": tilgang
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(mock_response).expect("Failed to serialize mock response"))
        .create_async()
        .await
}
