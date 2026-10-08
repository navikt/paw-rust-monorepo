use mockito::{Matcher, Mock, ServerGuard};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;

#[derive(PartialEq, Eq, Hash)]
pub struct KafkaKeyGenMockRequest {
    pub identitetsnummer: String,
}

impl KafkaKeyGenMockRequest {
    pub fn new(identititesnummer: &str) -> Self {
        Self {
            identitetsnummer: identititesnummer.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct KafkaKeyGenMockResponse {
    pub hent_key: KeyMockResponse,
    pub hent_ideniteter: IdentiteterMockResponse,
}

#[derive(Serialize)]
pub struct KeyMockResponse {
    pub id: i64,
    pub key: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentiteterMockResponse {
    pub record_key: i64,
    pub arbeidssoeker_id: i64,
    pub identiteter: Vec<MockIdentitet>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockIdentitet {
    pub identitet: String,
    #[serde(rename = "type")]
    pub identitet_type: String,
    pub gjeldende: bool,
}

pub struct KafkaKeyGenMockGuard {
    pub mocks: Vec<Mock>,
}

pub fn default_kafka_key_gen_mock_responses()
-> HashMap<KafkaKeyGenMockRequest, KafkaKeyGenMockResponse> {
    let arbeidssoeker_id = 12345i64;
    let aktor_id = "101701234500";
    let identiteter = vec!["01017012345", "41017012345"];
    let mut map = HashMap::new();
    map.insert(
        KafkaKeyGenMockRequest::new("01017012345"),
        KafkaKeyGenMockResponse {
            hent_key: key_mock_response(arbeidssoeker_id),
            hent_ideniteter: ideniteter_mock_response(arbeidssoeker_id, aktor_id, &identiteter),
        },
    );
    map.insert(
        KafkaKeyGenMockRequest::new("41017012345"),
        KafkaKeyGenMockResponse {
            hent_key: key_mock_response(arbeidssoeker_id),
            hent_ideniteter: ideniteter_mock_response(arbeidssoeker_id, aktor_id, &identiteter),
        },
    );
    map
}

fn key_mock_response(arbeidssoeker_id: i64) -> KeyMockResponse {
    KeyMockResponse {
        id: arbeidssoeker_id,
        key: -arbeidssoeker_id,
    }
}

fn ideniteter_mock_response(
    arbeidssoeker_id: i64,
    aktor_id: &str,
    identiteter: &Vec<&str>,
) -> IdentiteterMockResponse {
    let mut alle_identiteter = vec![];
    alle_identiteter.push(MockIdentitet {
        identitet: aktor_id.to_string(),
        identitet_type: "AKTORID".to_string(),
        gjeldende: true,
    });
    alle_identiteter.push(MockIdentitet {
        identitet: arbeidssoeker_id.to_string(),
        identitet_type: "ARBEIDSSOEKERID".to_string(),
        gjeldende: true,
    });
    for i in 0..identiteter.len() {
        alle_identiteter.push(MockIdentitet {
            identitet: identiteter[i].to_string(),
            identitet_type: "FOLKEREGISTERIDENT".to_string(),
            gjeldende: i == 0, // Sett den første identiteten som gjeldende
        });
    }
    IdentiteterMockResponse {
        record_key: -arbeidssoeker_id,
        arbeidssoeker_id,
        identiteter: alle_identiteter,
    }
}

pub async fn init_kafka_key_gen_api_mocks(
    mockito_server: &mut ServerGuard,
    mock_responses: HashMap<KafkaKeyGenMockRequest, KafkaKeyGenMockResponse>,
) -> Result<KafkaKeyGenMockGuard, Box<dyn Error>> {
    let _ = env_logger::try_init();
    let mut mocks = vec![];
    for (mock_request, mock_response) in &mock_responses {
        mocks.push(
            hent_eller_opprett_api_mock(mockito_server, mock_request, &mock_response.hent_key)
                .await,
        );
        mocks.push(
            identiteter_api_mock(mockito_server, mock_request, &mock_response.hent_ideniteter)
                .await,
        );
    }

    Ok(KafkaKeyGenMockGuard { mocks })
}

async fn hent_eller_opprett_api_mock(
    mockito_server: &mut ServerGuard,
    mock_request: &KafkaKeyGenMockRequest,
    mock_response: &KeyMockResponse,
) -> Mock {
    let identitetsnummer = &mock_request.identitetsnummer;
    mockito_server
        .mock("POST", "/api/v2/hentEllerOpprett")
        .match_body(Matcher::PartialJson(json!({
            "ident": identitetsnummer
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(mock_response).expect("Failed to serialize mock response"))
        .create_async()
        .await
}

async fn identiteter_api_mock(
    mockito_server: &mut ServerGuard,
    mock_request: &KafkaKeyGenMockRequest,
    mock_response: &IdentiteterMockResponse,
) -> Mock {
    let identitetsnummer = &mock_request.identitetsnummer;
    mockito_server
        .mock("POST", "/api/v2/identiteter")
        .match_body(Matcher::PartialJson(json!({
            "identitet": identitetsnummer
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(mock_response).expect("Failed to serialize mock response"))
        .create_async()
        .await
}
