use crate::parse::{enum_type_not_found, EnumTypeParseError};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KontorType {
    Arena,
    GeografiskTilknytning,
    #[serde(other)]
    #[default]
    Arbeidsoppfolging,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kontor {
    pub kontor_id: String,
    pub kontor_navn: String,
    #[serde(default = "default_kontor_type")]
    #[serde(skip_serializing)] // Felt finnes ikke i hendelsene. Lagt til for convenience.
    pub kontor_type: KontorType,
}

fn default_kontor_type() -> KontorType {
    KontorType::Arbeidsoppfolging
}
