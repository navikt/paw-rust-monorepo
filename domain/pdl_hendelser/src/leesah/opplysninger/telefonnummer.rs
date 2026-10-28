use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Telefonnummer {
    pub landskode: Option<String>,
    pub nummer: Option<String>,
    pub prioritet: Option<i32>,
}
