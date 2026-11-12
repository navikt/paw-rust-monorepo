use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TiltakshistorikkOpphav {
    Arena,
    TeamKomet,
    TeamTiltak,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}
