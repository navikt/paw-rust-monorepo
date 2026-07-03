use crate::vo::ja_nei_vet_ikke::JaNeiVetIkke;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Utdanning {
    pub nus: String,
    pub bestaatt: Option<JaNeiVetIkke>,
    pub godkjent: Option<JaNeiVetIkke>,
}
