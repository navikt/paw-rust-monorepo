use serde::{Deserialize, Serialize};

use super::{BrukerType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bruker {
    #[serde(rename = "type")]
    pub bruker_type: BrukerType,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sikkerhetsnivaa: Option<String>,
}
