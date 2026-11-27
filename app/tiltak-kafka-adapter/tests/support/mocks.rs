use crate::support::test_data::TestData;
use chrono::Utc;
use mockito::{Matcher, Mock, Server, ServerGuard};
use mulighetsrommet::tiltakshistorikk::TiltakshistorikkV1Response;
use mulighetsrommet::vo::{
    Deltakelse, TeamTiltakAvtale, TeamTiltakStatus, TeamTiltakTiltakskode, TeamTiltakTiltakstype,
    Virksomhet,
};
use paw_kafka_key_gen_api_mock::{
    KafkaKeyGenMockExchange, KafkaKeyGenMockGuard, KafkaKeyGenMockResponse,
    create_ideniteter_mock_response, init_kafka_key_gen_api_mocks,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::error::Error;
use uuid::Uuid;

pub struct Mocks {
    pub server: ServerGuard,
    _key_gen_mocks: KafkaKeyGenMockGuard,
    _mulighetsrommet_mocks: MulighetsrommetMockGuard,
}

pub struct ErrorRequest {
    pub ident: String,
    pub status: usize,
}

impl ErrorRequest {
    pub fn new(ident: &str, status: usize) -> Self {
        Self {
            ident: ident.to_string(),
            status,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn create_kafka_key_gen_error_response(request: &ErrorRequest) -> KafkaKeyGenMockExchange {
    KafkaKeyGenMockExchange {
        ident: request.ident.to_string(),
        status: request.status,
        response: KafkaKeyGenMockResponse::Identiteter {
            payload: serde_json::to_value(ErrorResponse {
                error: "BOOM!".to_string(),
            })
            .expect("Failed to serialize mock response"),
        },
    }
}

pub fn create_mulighetsrommet_error_response(
    request: &ErrorRequest,
) -> MulighetsrommetMockExchange {
    MulighetsrommetMockExchange {
        ident: request.ident.to_string(),
        status: request.status,
        response: MulighetsrommetMockResponse::Tiltakshistorikk {
            payload: serde_json::to_value(ErrorResponse {
                error: "BOOM!".to_string(),
            })
            .expect("Failed to serialize mock response"),
        },
    }
}

pub async fn create_mocks_from_test_data(test_data: &Vec<TestData>) -> Mocks {
    let kafka_key_gen_exchanges: Vec<KafkaKeyGenMockExchange> = test_data
        .iter()
        .map(|f| create_kafka_key_gen_ok_response(f.identitetsnummer.clone()))
        .collect();
    let mulighetsrommet_exchanges: Vec<MulighetsrommetMockExchange> = test_data
        .iter()
        .map(|f| create_mulighetsrommet_ok_response(f.identitetsnummer.clone()))
        .collect();

    create_mocks(kafka_key_gen_exchanges, mulighetsrommet_exchanges).await
}

pub async fn create_mocks_from_http_status(requests: Vec<ErrorRequest>) -> Mocks {
    let kafka_key_gen_exchanges: Vec<KafkaKeyGenMockExchange> = requests
        .iter()
        .map(create_kafka_key_gen_error_response)
        .collect();
    let mulighetsrommet_exchanges: Vec<MulighetsrommetMockExchange> = requests
        .iter()
        .map(create_mulighetsrommet_error_response)
        .collect();

    create_mocks(kafka_key_gen_exchanges, mulighetsrommet_exchanges).await
}

pub async fn create_mocks(
    kafka_key_gen_exchanges: Vec<KafkaKeyGenMockExchange>,
    mulighetsrommet_exchanges: Vec<MulighetsrommetMockExchange>,
) -> Mocks {
    let mut mockito_server = Server::new_async().await;
    let key_gen_mocks = init_kafka_key_gen_api_mocks(&mut mockito_server, kafka_key_gen_exchanges)
        .await
        .expect("Mock feilet");
    let mulighetsrommet_mocks =
        init_mulighetsrommet_api_mocks(&mut mockito_server, mulighetsrommet_exchanges)
            .await
            .expect("Mock feilet");

    Mocks {
        server: mockito_server,
        _key_gen_mocks: key_gen_mocks,
        _mulighetsrommet_mocks: mulighetsrommet_mocks,
    }
}

pub struct MulighetsrommetMockExchange {
    pub ident: String,
    pub status: usize,
    pub response: MulighetsrommetMockResponse,
}

pub enum MulighetsrommetMockResponse {
    Tiltakshistorikk { payload: Value },
}

pub struct MulighetsrommetMockGuard {
    pub _mocks: Vec<Mock>,
}

pub async fn init_mulighetsrommet_api_mocks(
    mockito_server: &mut ServerGuard,
    exchanges: Vec<MulighetsrommetMockExchange>,
) -> Result<MulighetsrommetMockGuard, Box<dyn Error>> {
    let mut mocks = vec![];
    for exchange in exchanges {
        match &exchange.response {
            MulighetsrommetMockResponse::Tiltakshistorikk { payload } => {
                let identer = vec![exchange.ident.clone()];
                let status = exchange.status;
                mocks.push(post_historikk_api_mock(mockito_server, identer, status, payload).await);
            }
        }
    }
    Ok(MulighetsrommetMockGuard { _mocks: mocks })
}

async fn post_historikk_api_mock<T: Sized + Serialize>(
    mockito_server: &mut ServerGuard,
    identer: Vec<String>,
    status: usize,
    payload: T,
) -> Mock {
    let body = serde_json::to_string(&payload).expect("Failed to serialize mock response");
    mockito_server
        .mock("POST", "/api/v1/historikk")
        .match_body(Matcher::PartialJson(json!({
            "identer": identer
        })))
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await
}

pub fn create_kafka_key_gen_ok_response(ident: String) -> KafkaKeyGenMockExchange {
    let arbeidssoeker_id = 12345i64;
    let aktor_id = "101701234500";
    let identer = vec![ident.as_str()];

    KafkaKeyGenMockExchange {
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
    }
}

pub fn create_mulighetsrommet_ok_response(identitetsnummer: String) -> MulighetsrommetMockExchange {
    MulighetsrommetMockExchange {
        ident: identitetsnummer.clone(),
        status: 200,
        response: create_tiltakshistorikk_response(identitetsnummer.clone()),
    }
}

fn create_tiltakshistorikk_response(identitetsnummer: String) -> MulighetsrommetMockResponse {
    MulighetsrommetMockResponse::Tiltakshistorikk {
        payload: serde_json::to_value(TiltakshistorikkV1Response {
            historikk: vec![create_team_tiltak_avtale(identitetsnummer)],
        })
        .expect("Failed to serialize mock response"),
    }
}

fn create_team_tiltak_avtale(identitetsnummer: String) -> Deltakelse {
    Deltakelse::TeamTiltakAvtale(TeamTiltakAvtale {
        norsk_ident: identitetsnummer,
        start_dato: None,
        slutt_dato: None,
        opprettet_tidspunkt: Utc::now().to_string(),
        oppdatert_tidspunkt: Utc::now().to_string(),
        id: Uuid::new_v4().to_string(),
        tittel: "Team Tiltak Avtale".to_string(),
        tiltakstype: TeamTiltakTiltakstype {
            tiltakskode: TeamTiltakTiltakskode::Arbeidstrening,
            navn: "Testtiltak".to_string(),
        },
        status: TeamTiltakStatus::Paabegynt,
        stillingsprosent: None,
        dager_per_uke: None,
        arbeidsgiver: Virksomhet {
            organisasjonsnummer: "123456789".to_string(),
            navn: Some("Testbedrift".to_string()),
        },
        opphav: None,
    })
}
