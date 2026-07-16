use crate::kontor::Kontor;
use crate::parse::{enum_type_not_found, EnumTypeParseError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use uuid::Uuid;

pub const POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC: &'static str = "poao.siste-oppfolgingsperiode-v3";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SisteEndringsType {
    OppfolgingStartet,
    ArbeidsoppfolgingskontorEndret,
    OppfolgingAvsluttet,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "sisteEndringsType")]
pub enum Oppfolgingsperiode {
    #[serde(rename = "OPPFOLGING_STARTET")]
    Startet(OppfolgingsperiodeEndret),
    #[serde(rename = "ARBEIDSOPPFOLGINGSKONTOR_ENDRET")]
    Endret(OppfolgingsperiodeEndret),
    #[serde(rename = "OPPFOLGING_AVSLUTTET")]
    Avsluttet(OppfolgingsperiodeAvsluttet),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppfolgingsperiodeEndret {
    #[serde(rename = "oppfolgingsperiodeUuid")]
    pub id: Uuid,
    pub aktor_id: String,
    pub ident: String,
    pub kontor: Kontor,
    pub start_tidspunkt: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppfolgingsperiodeAvsluttet {
    #[serde(rename = "oppfolgingsperiodeUuid")]
    pub id: Uuid,
    pub aktor_id: String,
    pub ident: String,
    pub start_tidspunkt: DateTime<Utc>,
    pub slutt_tidspunkt: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oppfolgingsperiode_startet() {
        let source_json = r#"
        {
            "sisteEndringsType": "OPPFOLGING_STARTET",
            "oppfolgingsperiodeUuid": "366004e8-9bfc-47bb-8469-7818bc21b6df",
            "aktorId": "12345",
            "ident": "01017012345",
            "kontor": {
                "kontorId": "1234",
                "kontorNavn": "Kontor-1"
            },
            "startTidspunkt": "2026-07-01T13:37:00Z"
        }
        "#;
        let oppfolgingsperiode: Oppfolgingsperiode = serde_json::from_str(source_json).unwrap();
        let target_json = serde_json::to_string(&oppfolgingsperiode).unwrap();

        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(data) => {
                assert_eq!(data.id.to_string(), "366004e8-9bfc-47bb-8469-7818bc21b6df");
                assert_eq!(data.aktor_id, "12345");
                assert_eq!(data.ident, "01017012345");
                assert_eq!(data.kontor.kontor_id, "1234");
                assert_eq!(data.kontor.kontor_navn, "Kontor-1");
                assert_eq!(
                    data.start_tidspunkt.to_rfc3339(),
                    "2026-07-01T13:37:00+00:00"
                );
                assert_eq!(
                    target_json,
                    source_json
                        .replace(" ", "")
                        .replace("\t", "")
                        .replace("\n", "")
                );
            }
            Oppfolgingsperiode::Endret(_) => {
                panic!("Feil type")
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Feil type")
            }
        }
    }

    #[test]
    fn test_oppfolgingsperiode_endret() {
        let source_json = r#"
        {
            "sisteEndringsType": "ARBEIDSOPPFOLGINGSKONTOR_ENDRET",
            "oppfolgingsperiodeUuid": "366004e8-9bfc-47bb-8469-7818bc21b6df",
            "aktorId": "12345",
            "ident": "01017012345",
            "kontor": {
                "kontorId": "4321",
                "kontorNavn": "Kontor-2"
            },
            "startTidspunkt": "2026-07-01T13:37:00Z"
        }
        "#;
        let oppfolgingsperiode: Oppfolgingsperiode = serde_json::from_str(source_json).unwrap();
        let target_json = serde_json::to_string(&oppfolgingsperiode).unwrap();

        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(_) => {
                panic!("Feil type")
            }
            Oppfolgingsperiode::Endret(data) => {
                assert_eq!(data.id.to_string(), "366004e8-9bfc-47bb-8469-7818bc21b6df");
                assert_eq!(data.aktor_id, "12345");
                assert_eq!(data.ident, "01017012345");
                assert_eq!(data.kontor.kontor_id, "4321");
                assert_eq!(data.kontor.kontor_navn, "Kontor-2");
                assert_eq!(
                    data.start_tidspunkt.to_rfc3339(),
                    "2026-07-01T13:37:00+00:00"
                );
                assert_eq!(
                    target_json,
                    source_json
                        .replace(" ", "")
                        .replace("\t", "")
                        .replace("\n", "")
                );
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Feil type")
            }
        }
    }

    #[test]
    fn test_oppfolgingsperiode_avsluttet() {
        let source_json = r#"
        {
            "sisteEndringsType": "OPPFOLGING_AVSLUTTET",
            "oppfolgingsperiodeUuid": "366004e8-9bfc-47bb-8469-7818bc21b6df",
            "aktorId": "12345",
            "ident": "01017012345",
            "startTidspunkt": "2026-07-01T13:37:00Z",
            "sluttTidspunkt":"2026-07-10T13:37:00Z"
        }
        "#;
        let oppfolgingsperiode: Oppfolgingsperiode = serde_json::from_str(source_json).unwrap();
        let target_json = serde_json::to_string(&oppfolgingsperiode).unwrap();

        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(data) => {
                panic!("Feil type")
            }
            Oppfolgingsperiode::Endret(_) => {
                panic!("Feil type")
            }
            Oppfolgingsperiode::Avsluttet(data) => {
                assert_eq!(data.id.to_string(), "366004e8-9bfc-47bb-8469-7818bc21b6df");
                assert_eq!(data.aktor_id, "12345");
                assert_eq!(data.ident, "01017012345");
                assert_eq!(
                    data.start_tidspunkt.to_rfc3339(),
                    "2026-07-01T13:37:00+00:00"
                );
                assert_eq!(
                    data.slutt_tidspunkt.to_rfc3339(),
                    "2026-07-10T13:37:00+00:00"
                );
                assert_eq!(
                    target_json,
                    source_json
                        .replace(" ", "")
                        .replace("\t", "")
                        .replace("\n", "")
                );
            }
        }
    }
}
