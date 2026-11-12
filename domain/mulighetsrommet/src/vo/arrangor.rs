use super::Virksomhet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arrangor {
    pub hovedenhet: Option<Virksomhet>,
    pub underenhet: Virksomhet,
}
