use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Familierelasjon {
    pub relatert_persons_ident: String,
    pub relatert_persons_rolle: String,
    pub min_rolle_for_person: Option<String>,
}
