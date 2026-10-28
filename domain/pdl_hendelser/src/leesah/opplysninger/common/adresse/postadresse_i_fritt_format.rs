use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostadresseIFrittFormat {
    pub adresselinje1: Option<String>,
    pub adresselinje2: Option<String>,
    pub adresselinje3: Option<String>,
    pub postnummer: Option<String>,
}
