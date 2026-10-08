use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TilgangRequest {
    pub identitetsnummer: String,
    pub nav_ansatt_id: String,
    pub tilgang: TilgangType,
}

impl TilgangRequest {
    pub fn new(identitetsnummer: String, nav_ansatt_id: String, tilgang: TilgangType) -> Self {
        Self {
            identitetsnummer,
            nav_ansatt_id,
            tilgang,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TilgangResponse {
    pub har_tilgang: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr)]
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
