use std::collections::hash_set;

use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use interne_hendelser::{
    Startet,
    vo::{Bruker as HendelseBruker, Metadata as HendelseMetadata, Opplysning},
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use texas_client::{response::TokenResponse, token_client::M2MTokenClient};
use utgang::kafka::periode_deserializer::{Bruker, BrukerType, Metadata, Periode};
use uuid::Uuid;

pub struct StubTokenClient;

#[async_trait]
impl M2MTokenClient for StubTokenClient {
    async fn get_token(&self, _target: String) -> Result<TokenResponse> {
        Ok(TokenResponse {
            access_token: "stub-token".to_string(),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }
}

pub fn lag_person_json(
    foedselsdato: &str,
    kommunenummer: &str,
    statsborgerskap: &str,
    freg_status: &str,
) -> Value {
    json!({
        "foedselsdato": [{ "foedselsdato": foedselsdato, "foedselsaar": null }],
        "statsborgerskap": [{ "land": statsborgerskap, "metadata": { "endringer": [] } }],
        "opphold": [],
        "folkeregisterpersonstatus": [{ "forenkletStatus": freg_status, "metadata": { "endringer": [] } }],
        "bostedsadresse": [{
            "angittFlyttedato": null,
            "gyldigFraOgMed": null,
            "gyldigTilOgMed": null,
            "vegadresse": { "kommunenummer": kommunenummer },
            "matrikkeladresse": null,
            "ukjentBosted": null,
            "utenlandskAdresse": null
        }],
        "innflyttingTilNorge": [],
        "utflyttingFraNorge": []
    })
}

pub fn lag_pdl_bolk_respons(items: Vec<(&str, Option<Value>)>) -> String {
    let bolk: Vec<Value> = items
        .into_iter()
        .map(|(ident, person)| {
            json!({
                "ident": ident,
                "person": person,
                "code": "ok"
            })
        })
        .collect();
    json!({ "data": { "hentPersonBolk": bolk } }).to_string()
}

pub async fn sett_gammel_sist_oppdatert(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
) {
    sqlx::query(
        "UPDATE periode SET sist_oppdatert_timestamp = $1 WHERE id = $2",
    )
    .bind(chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z").unwrap().to_utc())
    .bind(periode_id)
    .execute(&mut **tx)
    .await
    .expect("Failed to backdate sist_oppdatert_timestamp");
}

pub fn main_avro_periode() -> Periode {
    Periode {
        id: Uuid::new_v4(),
        identitetsnummer: "12345678901".to_string(),
        startet: main_avro_metadata(),
        avsluttet: Option::None,
    }
}

pub fn main_avro_metadata() -> Metadata {
    Metadata {
        tidspunkt: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .to_utc(),
        utfoert_av: main_avro_bruker(),
        kilde: "test-kilde".to_string(),
        aarsak: "test-aarsak".to_string(),
        tidspunkt_fra_kilde: Option::None,
    }
}

pub fn main_avro_bruker() -> Bruker {
    Bruker {
        id: "12345678910".to_string(),
        bruker_type: main_avro_bruker_type(),
        sikkerhetsnivaa: Option::None,
    }
}

pub fn main_avro_bruker_type() -> BrukerType {
    BrukerType::Sluttbruker
}

pub fn hendelse_startet() -> Startet {
    Startet {
        hendelse_id: Uuid::new_v4(),
        id: 1,
        identitetsnummer: "12345678901".to_string(),
        metadata: hendelse_metadata(),
        opplysninger: hash_set::HashSet::from([
            Opplysning::ErOver18Aar,
            Opplysning::HarNorskAdresse,
            Opplysning::BosattEtterFregLoven,
            Opplysning::SammeSomInnloggetBruker,
            Opplysning::SisteFlyttingVarInnTilNorge,
        ]),
    }
}

pub fn hendelse_metadata() -> HendelseMetadata {
    HendelseMetadata {
        tidspunkt: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .to_utc(),
        utfoert_av: hendelse_bruker(),
        kilde: "test-kilde".to_string(),
        aarsak: "test-aarsak".to_string(),
        tidspunkt_fra_kilde: Option::None,
    }
}

pub fn hendelse_bruker() -> HendelseBruker {
    HendelseBruker {
        id: "12345678910".to_string(),
        bruker_type: interne_hendelser::vo::BrukerType::Sluttbruker,
        sikkerhetsnivaa: Option::None,
    }
}
