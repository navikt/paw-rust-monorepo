use crate::brukertype::BrukerType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bruker {
    #[serde(rename = "type")]
    pub bruker_type: BrukerType,
    pub id: String,
    pub sikkerhetsnivaa: Option<String>,
}
