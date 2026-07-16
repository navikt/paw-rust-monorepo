use chrono::NaiveDate;
use mockito::{Matcher, Mock, ServerGuard};
use serde_json::json;
use std::error::Error;

pub struct PdlMockResponse {
    pub match_ident: String,
    pub hent_identer: Vec<PdlMockIdent>,
    pub hent_person: Vec<PdlMockPerson>,
    pub hent_person_navn: Vec<PdlMockPersonNavn>,
}

pub struct PdlMockIdent {
    pub ident: String,
    pub gruppe: String,
    pub historisk: bool,
}

pub struct PdlMockPerson {
    pub foedselsdato: NaiveDate,
    pub folkeregisterpersonstatus: String,
    pub kommunenummer: String,
}

pub struct PdlMockPersonNavn {
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
}

pub struct PdlMockGuard {
    pub mocks: Vec<Mock>,
}

pub fn default_pdl_mock_responses() -> Vec<PdlMockResponse> {
    let identer = vec!["01017012345", "41017012345"];
    vec![
        PdlMockResponse {
            match_ident: "01017012345".to_string(),
            hent_identer: hent_identer_mock(&identer),
            hent_person: hent_person_mock(),
            hent_person_navn: hent_person_navn_mock(),
        },
        PdlMockResponse {
            match_ident: "41017012345".to_string(),
            hent_identer: hent_identer_mock(&identer),
            hent_person: hent_person_mock(),
            hent_person_navn: hent_person_navn_mock(),
        },
    ]
}

fn hent_identer_mock(identer: &Vec<&str>) -> Vec<PdlMockIdent> {
    let mut alle_identiteter = vec![];
    for i in 0..identer.len() {
        alle_identiteter.push(PdlMockIdent {
            ident: identer[i].to_string(),
            gruppe: "FOLKEREGISTERIDENT".to_string(),
            historisk: i != 0, // Sett den første identiteten som gjeldende
        });
    }
    alle_identiteter
}

fn hent_person_mock() -> Vec<PdlMockPerson> {
    vec![PdlMockPerson {
        foedselsdato: NaiveDate::from_ymd_opt(1970, 1, 1).expect("Kunne ikke lage NaiveDate"),
        folkeregisterpersonstatus: "bosattEtterFolkeregisterloven".to_string(),
        kommunenummer: "5501".to_string(),
    }]
}

fn hent_person_navn_mock() -> Vec<PdlMockPersonNavn> {
    vec![PdlMockPersonNavn {
        fornavn: "Ola".to_string(),
        mellomnavn: None,
        etternavn: "Nordmann".to_string(),
    }]
}

pub async fn init_pdl_mock(
    mockito_server: &mut ServerGuard,
    mock_responses: Vec<PdlMockResponse>,
) -> Result<PdlMockGuard, Box<dyn Error>> {
    let _ = env_logger::try_init();
    let mut mocks = vec![];
    for response in mock_responses {
        let match_ident = &response.match_ident;
        for ident in &response.hent_identer {
            mocks.push(ident_mock(mockito_server, match_ident, ident).await);
        }
        for person in &response.hent_person {
            mocks.push(person_mock(mockito_server, match_ident, person).await);
        }
        for person in &response.hent_person_navn {
            mocks.push(person_navn_mock(mockito_server, match_ident, person).await);
        }
    }

    Ok(PdlMockGuard { mocks })
}

async fn ident_mock(
    mockito_server: &mut ServerGuard,
    match_ident: &String,
    ident: &PdlMockIdent,
) -> Mock {
    mockito_server
        .mock("POST", "/pdl")
        .match_body(Matcher::PartialJson(json!({
            "operationName": "HentIdenter",
            "variables": {
                "ident": match_ident,
                "historisk": false
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/graphql-response+json")
        .with_body(
            json!({
                "data": {
                    "hentIdenter": {
                        "identer": [
                            {
                                "ident": ident.ident,
                                "gruppe": ident.gruppe,
                                "historisk": ident.historisk,
                            }
                        ]
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
    match_ident: &String,
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
                "ident": match_ident,
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
    match_ident: &String,
    person: &PdlMockPersonNavn,
) -> Mock {
    mockito_server
        .mock("POST", "/pdl")
        .match_body(Matcher::PartialJson(json!({
            "operationName": "HentPersonNavn",
            "variables": {
                "ident": match_ident,
                "historisk": false
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/graphql-response+json")
        .with_body(
            json!({
                "data": {
                    "hentPerson": {
                        "navn": [
                            {
                                "fornavn": person.fornavn,
                                "mellomnavn": person.mellomnavn,
                                "etternavn": person.etternavn,
                            }
                        ]
                    }
                }
            })
            .to_string(),
        )
        .create_async()
        .await
}
