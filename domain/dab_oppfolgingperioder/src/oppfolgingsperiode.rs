use crate::kontor::Kontor;
use crate::parse::{enum_type_not_found, EnumTypeParseError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

pub const SISTE_OPPFOLGINGSPERIODE_V3_TOPIC: &'static str = "poao.siste-oppfolgingsperiode-v3";

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
    Startet(OppfolgingsperiodeStartet),
    #[serde(rename = "ARBEIDSOPPFOLGINGSKONTOR_ENDRET")]
    Endret(OppfolgingsperiodeEndret),
    #[serde(rename = "OPPFOLGING_AVSLUTTET")]
    Avsluttet(OppfolgingsperiodeAvsluttet),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppfolgingsperiodeStartet {
    pub kontor: Kontor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppfolgingsperiodeEndret {
    pub kontor: Kontor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppfolgingsperiodeAvsluttet {
    pub slutt_tidspunkt: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kontor::KontorType;

    #[test]
    fn test_oppfolgingsperiode_startet() {
        let source_json = r#"{"sisteEndringsType":"OPPFOLGING_STARTET","kontor":{"kontorId":"1234","kontorNavn":"Dummy"}}"#;
        let oppfolgingsperiode: Oppfolgingsperiode = serde_json::from_str(source_json).unwrap();
        let target_json = serde_json::to_string(&oppfolgingsperiode).unwrap();

        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(data) => {
                assert_eq!(data.kontor.kontor_id, "1234");
                assert_eq!(data.kontor.kontor_navn, "Dummy");
                assert_eq!(data.kontor.kontor_type, KontorType::Arbeidsoppfolging);
                assert_eq!(target_json, source_json);
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
        let source_json = r#"{"sisteEndringsType":"ARBEIDSOPPFOLGINGSKONTOR_ENDRET","kontor":{"kontorId":"4321","kontorNavn":"Whatever"}}"#;
        let oppfolgingsperiode: Oppfolgingsperiode = serde_json::from_str(source_json).unwrap();
        let target_json = serde_json::to_string(&oppfolgingsperiode).unwrap();

        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(_) => {
                panic!("Feil type")
            }
            Oppfolgingsperiode::Endret(data) => {
                assert_eq!(data.kontor.kontor_id, "4321");
                assert_eq!(data.kontor.kontor_navn, "Whatever");
                assert_eq!(data.kontor.kontor_type, KontorType::Arbeidsoppfolging);
                assert_eq!(target_json, source_json);
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Feil type")
            }
        }
    }

    #[test]
    fn test_oppfolgingsperiode_avsluttet() {
        let source_json = r#"{"sisteEndringsType":"OPPFOLGING_AVSLUTTET","sluttTidspunkt":"2026-07-01T13:37:00Z"}"#;
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
                assert_eq!(
                    data.slutt_tidspunkt,
                    DateTime::parse_from_rfc3339("2026-07-01T13:37:00Z").unwrap()
                );
                assert_eq!(target_json, source_json);
            }
        }
    }
}
