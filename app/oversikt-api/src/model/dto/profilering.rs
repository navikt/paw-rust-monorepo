use crate::model::parse::{enum_type_not_found, EnumTypeParseError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{AsRefStr, EnumString};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profilering {
    pub id: Uuid,
    pub profilert_til: ProfilertTil,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr,
)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfilertTil {
    Udefinert,
    AntattGodeMuligheter,
    AntattBehovForVeiledning,
    OppgittHindringer,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

impl fmt::Display for ProfilertTil {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}
