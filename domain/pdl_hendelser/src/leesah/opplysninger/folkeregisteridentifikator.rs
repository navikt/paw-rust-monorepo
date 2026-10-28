use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folkeregisteridentifikator {
    pub identifikasjonsnummer: String,
    #[serde(rename = "type")]
    pub identifikator_type: String,
    pub status: String,
}
