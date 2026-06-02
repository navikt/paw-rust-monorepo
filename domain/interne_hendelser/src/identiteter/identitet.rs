use crate::identiteter::identitet_type::IdentitetType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Identitet {
    pub identitet: String,
    #[serde(rename = "type")]
    pub identitet_type: IdentitetType,
    pub gjeldende: bool,
}
