use serde::{Deserialize, Serialize};

use super::JaNeiVetIkke;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arbeidserfaring {
    pub har_hatt_arbeid: JaNeiVetIkke,
}
