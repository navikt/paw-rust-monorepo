use crate::model::parse::{enum_type_not_found, EnumTypeParseError};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr, ToSchema,
)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KontorType {
    Arbeidsoppfolging,
    Arena,
    GeografiskTilknytning,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TilknyttetKontor {
    pub kontor_id: String,
    pub kontor_navn: String,
    pub kontor_type: KontorType,
}
