use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr)]
pub enum IdentityProvider {
    #[serde(rename = "tokenx")]
    #[strum(serialize = "tokenx")]
    TokenX,
    #[serde(rename = "entra_id")]
    #[strum(serialize = "entra_id")]
    EntraId,
    #[serde(rename = "id_porten")]
    #[strum(serialize = "id_porten")]
    IdPorten,
    #[serde(rename = "maskinporten")]
    #[strum(serialize = "maskinporten")]
    Maskinporten,
}
