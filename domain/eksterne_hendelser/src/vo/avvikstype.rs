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
pub enum AvviksType {
    Forsinkelse,
    #[deprecated(note = "Use SLETTET instead")]
    Retting,
    Slettet,
    TidspunktKorrigert,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}
