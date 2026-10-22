use mockito::{Matcher, Mock, ServerGuard};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use strum::{AsRefStr, EnumString};

#[derive(Eq, PartialEq, Hash)]
pub struct NavAnsattPersonTilgangMockRequest {
    pub tilgang: TilgangType,
    pub nav_ident: String,
    pub identitetsnummer: String,
}

impl NavAnsattPersonTilgangMockRequest {
    pub fn new(tilgang: TilgangType, nav_ident: &str, identitetsnummer: &str) -> Self {
        Self {
            tilgang,
            nav_ident: nav_ident.to_string(),
            identitetsnummer: identitetsnummer.to_string(),
        }
    }
}

#[derive(Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattNavEnhetTilgangMockRequest {
    pub nav_ident: String,
    pub nav_enhet_id: String,
}

impl NavAnsattNavEnhetTilgangMockRequest {
    pub fn new(nav_ident: &str, nav_enhet_id: &str) -> Self {
        Self {
            nav_ident: nav_ident.to_string(),
            nav_enhet_id: nav_enhet_id.to_string(),
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

pub struct PawTilgangskontrollV2MockGuard {
    pub mocks: Vec<Mock>,
}

pub fn default_person_tilgang_mock_responses()
-> HashMap<NavAnsattPersonTilgangMockRequest, TilgangMockResponse> {
    let mut map = HashMap::new();
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Lese, "Z123456", "01017012345"),
        TilgangMockResponse::new(true),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Skrive, "Z123456", "01017012345"),
        TilgangMockResponse::new(true),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::LeseSkrive, "Z123456", "01017012345"),
        TilgangMockResponse::new(true),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Lese, "Z123456", "02017012345"),
        TilgangMockResponse::new(true),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Skrive, "Z123456", "02017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::LeseSkrive, "Z123456", "02017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Lese, "Z234567", "01017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Skrive, "Z234567", "01017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::LeseSkrive, "Z234567", "01017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Lese, "Z234567", "02017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::Skrive, "Z234567", "02017012345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattPersonTilgangMockRequest::new(TilgangType::LeseSkrive, "Z234567", "02017012345"),
        TilgangMockResponse::new(false),
    );
    map
}

pub fn default_enhet_tilgang_mock_responses()
-> HashMap<NavAnsattNavEnhetTilgangMockRequest, TilgangMockResponse> {
    let mut map = HashMap::new();
    map.insert(
        NavAnsattNavEnhetTilgangMockRequest::new("Z123456", "1234"),
        TilgangMockResponse::new(true),
    );
    map.insert(
        NavAnsattNavEnhetTilgangMockRequest::new("Z123456", "2345"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattNavEnhetTilgangMockRequest::new("Z234567", "1234"),
        TilgangMockResponse::new(false),
    );
    map.insert(
        NavAnsattNavEnhetTilgangMockRequest::new("Z234567", "2345"),
        TilgangMockResponse::new(true),
    );
    map
}

pub async fn init_paw_tilgangskontroll_v2_api_mocks(
    mockito_server: &mut ServerGuard,
    person_tilgang_mock_responses: HashMap<NavAnsattPersonTilgangMockRequest, TilgangMockResponse>,
    enhet_tilgang_mock_responses: HashMap<NavAnsattNavEnhetTilgangMockRequest, TilgangMockResponse>,
) -> Result<PawTilgangskontrollV2MockGuard, Box<dyn Error>> {
    let _ = env_logger::try_init();
    let mut mocks = vec![];
    for (mock_request, mock_response) in &person_tilgang_mock_responses {
        mocks.push(person_tilgang_api_mock(mockito_server, mock_request, mock_response).await);
    }
    for (mock_request, mock_response) in &enhet_tilgang_mock_responses {
        mocks.push(enhet_tilgang_api_mock(mockito_server, mock_request, mock_response).await);
    }

    Ok(PawTilgangskontrollV2MockGuard { mocks })
}

async fn person_tilgang_api_mock(
    mockito_server: &mut ServerGuard,
    mock_request: &NavAnsattPersonTilgangMockRequest,
    mock_response: &TilgangMockResponse,
) -> Mock {
    let tilgang = &mock_request.tilgang.as_ref().to_string();
    let nav_ident = &mock_request.nav_ident;
    let identitetsnummer = &mock_request.identitetsnummer;
    mockito_server
        .mock("POST", "/api/v1/tilgang/person")
        .match_body(Matcher::PartialJson(json!({
            "tilgang": tilgang,
            "navIdent": nav_ident,
            "identitetsnummer": identitetsnummer
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(mock_response).expect("Failed to serialize mock response"))
        .create_async()
        .await
}

async fn enhet_tilgang_api_mock(
    mockito_server: &mut ServerGuard,
    mock_request: &NavAnsattNavEnhetTilgangMockRequest,
    mock_response: &TilgangMockResponse,
) -> Mock {
    let nav_ident = &mock_request.nav_ident;
    let nav_enhet_id = &mock_request.nav_enhet_id;
    mockito_server
        .mock("POST", "/api/v1/tilgang/enhet")
        .match_body(Matcher::PartialJson(json!({
            "navIdent": nav_ident,
            "navEnhetId": nav_enhet_id
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(mock_response).expect("Failed to serialize mock response"))
        .create_async()
        .await
}
