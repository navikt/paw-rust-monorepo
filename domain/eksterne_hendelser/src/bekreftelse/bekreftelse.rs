use crate::bekreftelse::bekreftelsesloesning::Bekreftelsesloesning;
use crate::bekreftelse::svar::Svar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bekreftelse {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub bekreftelsesloesning: Bekreftelsesloesning,
    pub svar: Svar,
}

#[cfg(test)]
mod tests {}
