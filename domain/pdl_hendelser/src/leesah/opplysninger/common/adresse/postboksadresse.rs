use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Postboksadresse {
    pub postbokseier: Option<String>,
    pub postboks: String,
    pub postnummer: Option<String>,
}
