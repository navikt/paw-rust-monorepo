use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyRequest {
    pub ident: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyResponse {
    pub id: i64,
    pub key: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdentitetRequest {
    pub identitet: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdentitetResponse {
    pub record_key: Option<i64>,
    pub arbeidssoeker_id: Option<i64>,
    pub identiteter: Vec<Identitet>,
    pub pdl_identiteter: Option<Vec<Identitet>>,
    pub konflikter: Option<Vec<Konflikt>>,
}

impl IdentitetResponse {
    fn har_konflikter(&self) -> bool {
        self.konflikter.is_some() && self.konflikter.as_ref().unwrap().len() > 0
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Identitet {
    pub identitet: String,
    #[serde(rename = "type")]
    pub identitet_type: IdentitetType,
    pub gjeldende: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdentitetType {
    Folkeregisterident,
    Aktorid,
    Npid,
    Arbeidssoekerid,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Konflikt {
    #[serde(rename = "type")]
    pub konflikt_type: KonfliktType,
    pub detaljer: Option<KonfliktDetaljer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KonfliktType {
    Merge,
    Splitt,
    Slett,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KonfliktDetaljer {
    pub aktor_id_liste: Vec<String>,
    pub arbeidssoeker_id_liste: Vec<String>,
}

pub fn enum_type_not_found(type_: &str) -> EnumTypeParseError {
    EnumTypeParseError::UkjentType(type_.to_string())
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EnumTypeParseError {
    #[error("Ukjent enum: {0}")]
    UkjentType(String),
}
