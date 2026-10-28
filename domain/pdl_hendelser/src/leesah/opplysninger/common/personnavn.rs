use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Personnavn {
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
}
