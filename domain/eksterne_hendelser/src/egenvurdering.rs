use crate::metadata::Metadata;
use crate::profilert_til::ProfilertTil;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Egenvurdering {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub profilering_id: Uuid,
    pub sendt_inn_av: Metadata,
    pub profilert_til: ProfilertTil,
    pub egenvurdering: ProfilertTil,
}

#[cfg(test)]
mod tests {}
