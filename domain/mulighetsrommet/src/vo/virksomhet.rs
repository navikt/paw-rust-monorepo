use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Virksomhet {
    pub organisasjonsnummer: String,
    pub navn: Option<String>,
}
