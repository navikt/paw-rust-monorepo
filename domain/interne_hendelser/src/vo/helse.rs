use serde::{Deserialize, Serialize};

use super::JaNeiVetIkke;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Helse {
    pub helsetilstand_hindrer_arbeid: JaNeiVetIkke,
}
