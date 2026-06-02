use crate::metadata::Metadata;
use crate::profilert_til::ProfilertTil;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profilering {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub opplysninger_om_arbeidssoker_id: Uuid,
    pub sendt_inn_av: Metadata,
    pub profilert_til: ProfilertTil,
    pub jobbet_sammenhengende_seks_av_tolv_siste_mnd: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alder: Option<i32>,
}

#[cfg(test)]
mod tests {}
