use serde::{Deserialize, Serialize};

use super::JaNeiVetIkke;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Utdanning {
    pub nus: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bestaatt: Option<JaNeiVetIkke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub godkjent: Option<JaNeiVetIkke>,
}
