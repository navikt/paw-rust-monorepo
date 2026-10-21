use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr)]
pub enum TilgangType {
    #[strum(serialize = "LESE")]
    #[serde(rename = "LESE")]
    Lese,
    #[strum(serialize = "SKRIVE")]
    #[serde(rename = "SKRIVE")]
    Skrive,
    #[strum(serialize = "LESE_SKRIVE")]
    #[serde(rename = "LESE_SKRIVE")]
    LeseSkrive,
}
