use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kontor {
    pub kontor_id: String,
    pub kontor_navn: String,
}
