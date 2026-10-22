use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattPersonTilgangRequest {
    pub tilgang: TilgangType,
    pub nav_ident: String,
    pub identitetsnummer: String,
}

impl NavAnsattPersonTilgangRequest {
    pub fn new(tilgang: TilgangType, nav_ident: String, identitetsnummer: String) -> Self {
        Self {
            tilgang,
            nav_ident,
            identitetsnummer,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattNavEnhetTilgangRequest {
    pub nav_ident: String,
    pub nav_enhet_id: String,
}

impl NavAnsattNavEnhetTilgangRequest {
    pub fn new(nav_ident: String, nav_enhet_id: String) -> Self {
        Self {
            nav_ident,
            nav_enhet_id,
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
