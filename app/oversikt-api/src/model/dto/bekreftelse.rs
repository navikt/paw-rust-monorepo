use crate::model::parse::{enum_type_not_found, EnumTypeParseError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{AsRefStr, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Bekreftelse {
    pub id: Uuid,
    pub gjelder_fra: DateTime<Utc>,
    pub gjelder_til: DateTime<Utc>,
    pub har_jobbet: bool,
    pub vil_fortsette: bool,
    pub bekreftelsesloesning: Bekreftelsesloesning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr, ToSchema)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Bekreftelsesloesning {
    UkjentVerdi,
    Arbeidssoekerregisteret,
    Dagpenger,
    FriskmeldtTilArbeidsformidling,
}

impl fmt::Display for Bekreftelsesloesning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}
