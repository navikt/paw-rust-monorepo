use mockito::{Matcher, Mock, ServerGuard};
use serde::Serialize;
use serde_json::{Value, json};
use std::error::Error;

pub struct KafkaKeyGenMockExchange {
    pub ident: String,
    pub status: usize,
    pub response: KafkaKeyGenMockResponse,
}

pub enum KafkaKeyGenMockResponse {
    KafkaKeys { payload: Value },
    Identiteter { payload: Value },
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

pub fn default_kafka_key_gen_mock_responses() -> Vec<KafkaKeyGenMockExchange> {
    let arbeidssoeker_id = 12345i64;
    let aktor_id = "101701234500";
    let identer = vec!["01017012345", "41017012345"];
    let mut exchanges = Vec::new();

    for ident in identer.iter() {
        exchanges.push(KafkaKeyGenMockExchange {
            ident: ident.to_string(),
            status: 200,
            response: KafkaKeyGenMockResponse::KafkaKeys {
                payload: serde_json::to_value(create_kafka_key_mock_response(arbeidssoeker_id))
                    .expect("Failed to serialize mock response"),
            },
        });
        exchanges.push(KafkaKeyGenMockExchange {
            ident: ident.to_string(),
            status: 200,
            response: KafkaKeyGenMockResponse::Identiteter {
                payload: serde_json::to_value(create_ideniteter_mock_response(
                    arbeidssoeker_id,
                    aktor_id,
                    &identer.clone(),
                ))
                .expect("Failed to serialize mock response"),
            },
        });
    }

    exchanges
}

pub fn create_kafka_key_mock_response(arbeidssoeker_id: i64) -> KeyMockResponse {
    KeyMockResponse {
        id: arbeidssoeker_id,
        key: -arbeidssoeker_id,
    }
}

pub fn create_ideniteter_mock_response(
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
    exchanges: Vec<KafkaKeyGenMockExchange>,
) -> Result<KafkaKeyGenMockGuard, Box<dyn Error>> {
    let _ = env_logger::try_init();
    let mut mocks = vec![];
    for exchange in exchanges {
        match &exchange.response {
            KafkaKeyGenMockResponse::KafkaKeys { payload } => {
                mocks.push(
                    hent_eller_opprett_api_mock(
                        mockito_server,
                        &exchange.ident,
                        exchange.status,
                        &payload,
                    )
                    .await,
                );
            }
            KafkaKeyGenMockResponse::Identiteter { payload } => {
                mocks.push(
                    identiteter_api_mock(
                        mockito_server,
                        &exchange.ident,
                        exchange.status,
                        &payload,
                    )
                    .await,
                );
            }
        }
    }

    Ok(KafkaKeyGenMockGuard { mocks })
}

async fn hent_eller_opprett_api_mock(
    mockito_server: &mut ServerGuard,
    ident: &String,
    status: usize,
    payload: &Value,
) -> Mock {
    mockito_server
        .mock("POST", "/api/v2/hentEllerOpprett")
        .match_body(Matcher::PartialJson(json!({
            "ident": ident
        })))
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&payload).expect("Failed to serialize mock response"))
        .create_async()
        .await
}

async fn identiteter_api_mock(
    mockito_server: &mut ServerGuard,
    ident: &String,
    status: usize,
    payload: &Value,
) -> Mock {
    mockito_server
        .mock("POST", "/api/v2/identiteter")
        .match_body(Matcher::PartialJson(json!({
            "identitet": ident
        })))
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&payload).expect("Failed to serialize mock response"))
        .create_async()
        .await
}
