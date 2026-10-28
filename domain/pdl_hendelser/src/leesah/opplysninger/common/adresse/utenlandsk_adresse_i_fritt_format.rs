use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtenlandskAdresseIFrittFormat {
    pub adresselinje1: Option<String>,
    pub adresselinje2: Option<String>,
    pub adresselinje3: Option<String>,
    pub postkode: Option<String>,
    pub by_eller_stedsnavn: Option<String>,
    pub landkode: String,
}
