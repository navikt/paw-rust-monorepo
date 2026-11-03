use chrono::NaiveDate;
use mockito::{Matcher, Mock, Request, ServerGuard};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::error::Error;

pub struct PdlMockResponses {
    pub hent_identer: HashMap<&'static str, Vec<PdlMockIdent>>,
    pub hent_person: HashMap<&'static str, PdlMockPerson>,
    pub hent_person_navn: HashMap<&'static str, PdlMockPersonNavn>,
}

pub struct PdlMockResponse {
    pub match_ident: String,
    pub hent_identer: Vec<PdlMockIdent>,
}

#[derive(Debug, Serialize)]
pub struct PdlMockIdent {
    pub ident: String,
    pub gruppe: String,
    pub historisk: bool,
}

impl PdlMockIdent {
    fn new(ident: &str, historisk: bool) -> Self {
        Self {
            ident: ident.to_string(),
            gruppe: "FOLKEREGISTERIDENT".to_string(),
            historisk,
        }
    }
}

pub struct PdlMockPerson {
    pub foedselsdato: NaiveDate,
    pub folkeregisterpersonstatus: String,
    pub kommunenummer: String,
}

impl PdlMockPerson {
    fn new(kommunenummer: &str) -> Self {
        Self {
            foedselsdato: NaiveDate::from_ymd_opt(1970, 1, 1).expect("Kunne ikke lage NaiveDate"),
            folkeregisterpersonstatus: "bosattEtterFolkeregisterloven".to_string(),
            kommunenummer: kommunenummer.to_string(),
        }
    }
}

pub struct PdlMockPersonNavn {
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
}

impl PdlMockPersonNavn {
    fn new(fornavn: &str, etternavn: &str) -> Self {
        Self {
            fornavn: fornavn.to_string(),
            mellomnavn: None,
            etternavn: etternavn.to_string(),
        }
    }
}

pub struct PdlMockGuard {
    pub mocks: Vec<Mock>,
}

pub fn default_pdl_mock_responses() -> PdlMockResponses {
    PdlMockResponses {
        hent_identer: default_pdl_mock_ident_responses(),
        hent_person: default_pdl_mock_person_responses(),
        hent_person_navn: default_pdl_mock_person_navn_responses(),
    }
}

pub fn default_pdl_mock_ident_responses() -> HashMap<&'static str, Vec<PdlMockIdent>> {
    HashMap::<&'static str, Vec<PdlMockIdent>>::from([
        (
            "01017012345",
            vec![
                PdlMockIdent::new("01017012345", false),
                PdlMockIdent::new("41017012345", true),
            ],
        ),
        (
            "41017012345",
            vec![
                PdlMockIdent::new("01017012345", false),
                PdlMockIdent::new("41017012345", true),
            ],
        ),
        (
            "02017012345",
            vec![
                PdlMockIdent::new("02017012345", false),
                PdlMockIdent::new("42017012345", true),
            ],
        ),
        (
            "42017012345",
            vec![
                PdlMockIdent::new("02017012345", false),
                PdlMockIdent::new("42017012345", true),
            ],
        ),
        (
            "03017012345",
            vec![
                PdlMockIdent::new("03017012345", false),
                PdlMockIdent::new("43017012345", true),
            ],
        ),
        (
            "43017012345",
            vec![
                PdlMockIdent::new("03017012345", false),
                PdlMockIdent::new("43017012345", true),
            ],
        ),
        (
            "04017012345",
            vec![
                PdlMockIdent::new("04017012345", false),
                PdlMockIdent::new("44017012345", true),
            ],
        ),
        (
            "44017012345",
            vec![
                PdlMockIdent::new("04017012345", false),
                PdlMockIdent::new("44017012345", true),
            ],
        ),
        (
            "05017012345",
            vec![
                PdlMockIdent::new("05017012345", false),
                PdlMockIdent::new("45017012345", true),
            ],
        ),
        (
            "45017012345",
            vec![
                PdlMockIdent::new("05017012345", false),
                PdlMockIdent::new("45017012345", true),
            ],
        ),
    ])
}

pub fn default_pdl_mock_person_responses() -> HashMap<&'static str, PdlMockPerson> {
    HashMap::<&'static str, PdlMockPerson>::from([
        ("01017012345", PdlMockPerson::new("5501")),
        ("41017012345", PdlMockPerson::new("5501")),
        ("02017012345", PdlMockPerson::new("5502")),
        ("42017012345", PdlMockPerson::new("5502")),
        ("03017012345", PdlMockPerson::new("5503")),
        ("43017012345", PdlMockPerson::new("5503")),
        ("04017012345", PdlMockPerson::new("5504")),
        ("44017012345", PdlMockPerson::new("5504")),
        ("05017012345", PdlMockPerson::new("5505")),
        ("45017012345", PdlMockPerson::new("5505")),
    ])
}

pub fn default_pdl_mock_person_navn_responses() -> HashMap<&'static str, PdlMockPersonNavn> {
    HashMap::<&'static str, PdlMockPersonNavn>::from([
        ("01017012345", PdlMockPersonNavn::new("Anakin", "Skywalker")),
        ("41017012345", PdlMockPersonNavn::new("Darth", "Vader")),
        ("02017012345", PdlMockPersonNavn::new("Luke", "Skywalker")),
        ("42017012345", PdlMockPersonNavn::new("Luke", "Skywalker")),
        ("03017012345", PdlMockPersonNavn::new("Leia", "Organa")),
        ("43017012345", PdlMockPersonNavn::new("Leia", "Organa")),
        ("04017012345", PdlMockPersonNavn::new("Han", "Solo")),
        ("44017012345", PdlMockPersonNavn::new("Han", "Solo")),
        ("05017012345", PdlMockPersonNavn::new("Obiwan", "Kenobi")),
        ("45017012345", PdlMockPersonNavn::new("Obiwan", "Kenobi")),
    ])
}

pub async fn init_pdl_mock(
    mockito_server: &mut ServerGuard,
    mock_responses: PdlMockResponses,
) -> Result<PdlMockGuard, Box<dyn Error>> {
    let mut mocks = vec![];

    for (ident, identer) in mock_responses.hent_identer {
        mocks.push(ident_mock(mockito_server, &ident, &identer).await);
    }
    for (ident, person) in mock_responses.hent_person {
        mocks.push(person_mock(mockito_server, &ident, &person).await);
    }
    mocks.push(person_navn_mock(mockito_server, mock_responses.hent_person_navn).await);

    Ok(PdlMockGuard { mocks })
}

async fn ident_mock(
    mockito_server: &mut ServerGuard,
    ident: &str,
    mock_identer: &Vec<PdlMockIdent>,
) -> Mock {
    /*let identer = mock_identer
    .iter()
    .map(|mock_ident| {
        json!({
            "ident": mock_ident.ident,
            "gruppe": mock_ident.gruppe,
            "historisk": mock_ident.historisk,
        })
    })
    .collect::<Vec<Value>>();*/
    let identer = serde_json::to_value(mock_identer).unwrap();
    mockito_server
        .mock("POST", "/pdl")
        .match_body(Matcher::PartialJson(json!({
            "operationName": "HentIdenter",
            "variables": {
                "ident": ident,
                "historisk": false
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/graphql-response+json")
        .with_body(
            json!({
                "data": {
                    "hentIdenter": {
                        "identer": identer
                    }
                }
            })
            .to_string(),
        )
        .create_async()
        .await
}

async fn person_mock(
    mockito_server: &mut ServerGuard,
    ident: &str,
    person: &PdlMockPerson,
) -> Mock {
    let foedselsdato = person.foedselsdato.clone();
    let foedselsdato_string = foedselsdato.format("%Y-%m-%d").to_string();
    let foedselsaar_string = foedselsdato.format("%Y").to_string();
    let folkeregisterpersonstatus = person.folkeregisterpersonstatus.clone();
    let kommunenummer = person.kommunenummer.clone();
    mockito_server
        .mock("POST", "/pdl")
        .match_body(Matcher::PartialJson(json!({
            "operationName": "HentPerson",
            "variables": {
                "ident": ident,
                "historisk": false
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/graphql-response+json")
        .with_body(
            json!({
                "data": {
                    "hentPerson": {
                        "foedsel": [
                            {
                                "foedselsdato": foedselsdato_string,
                                "foedselsaar": foedselsaar_string
                            }
                        ],
                        "opphold": [
                            {
                                "oppholdFra": "2010-01-20",
                                "oppholdTil": "2022-01-20",
                                "type": "PERMANENT"
                            }
                        ],
                        "folkeregisterpersonstatus": [
                            {
                                "forenkletStatus": folkeregisterpersonstatus
                            }
                        ],
                        "bostedsadresse": [
                            {
                                "angittFlyttedato": "2010-01-20",
                                "gyldigFraOgMed": "2010-01-20",
                                "gyldigTilOgMed": "2022-01-20",
                                "vegadresse": {
                                    "kommunenummer": kommunenummer
                                },
                                "matrikkeladresse": null,
                                "ukjentBosted": null,
                                "utenlandskAdresse": null
                            }
                        ],
                        "innflyttingTilNorge": [],
                        "utflyttingFraNorge": []
                    }
                }
            })
            .to_string(),
        )
        .create_async()
        .await
}

async fn person_navn_mock(
    mockito_server: &mut ServerGuard,
    navn_per_ident: HashMap<&'static str, PdlMockPersonNavn>,
) -> Mock {
    mockito_server
        .mock("POST", "/pdl")
        .match_body(Matcher::PartialJson(json!({
            "operationName": "HentPersonNavn",
            "variables": {
                "historisk": false
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/graphql-response+json")
        .with_body_from_request(move |request| {
            let optional_ident = ident_from_request(request);
            let optional_person =
                optional_ident.and_then(|ident| navn_per_ident.get(ident.as_str()));
            let navn = optional_person
                .map(|person| {
                    json!([{
                        "fornavn": person.fornavn,
                        "mellomnavn": person.mellomnavn,
                        "etternavn": person.etternavn,
                    }])
                })
                .unwrap_or_else(|| json!([]));
            json!({
                "data": {
                    "hentPerson": {
                        "navn": navn
                    }
                }
            })
            .to_string()
            .into_bytes()
        })
        .create_async()
        .await
}

fn ident_from_request<'a>(request: &Request) -> Option<String> {
    request
        .body()
        .ok()
        .and_then(|body| serde_json::from_slice::<Value>(body).ok())
        .and_then(ident_from_request_body)
}

fn ident_from_request_body(body: Value) -> Option<String> {
    body.pointer("/variables/ident")
        .and_then(Value::as_str)
        .map(|ident| ident.to_string())
}
