use serde::{Deserialize, Serialize};

use super::JaNeiVetIkke;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub andre_forhold_hindrer_arbeid: Option<JaNeiVetIkke>,
}
