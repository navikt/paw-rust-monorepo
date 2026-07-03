use crate::vo::ja_nei_vet_ikke::JaNeiVetIkke;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annet {
    pub andre_forhold_hindrer_arbeid: Option<JaNeiVetIkke>,
}
