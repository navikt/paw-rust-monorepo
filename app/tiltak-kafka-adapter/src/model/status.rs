use crate::model::parse::{EnumTypeParseError, enum_type_not_found};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr, sqlx::Type)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SynkStatus {
    Venter,
    Aktiv,
    Ferdig,
    Feilet,
    Avbrutt,
}

impl fmt::Display for SynkStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}
