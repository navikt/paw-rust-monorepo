use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use crate::aarsak::Aarsak;
use crate::vo::{Metadata, Opplysning};

pub const AVSLUTTET_HENDELSE_TYPE: &str = "intern.v1.avsluttet";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avsluttet {
    pub hendelse_id: Uuid,
    pub id: i64,
    pub identitetsnummer: String,
    pub metadata: Metadata,
    #[serde(default)]
    pub opplysninger: HashSet<Opplysning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periode_id: Option<Uuid>,
    #[serde(default)]
    pub kalkulert_aarsak: Aarsak,
    #[serde(default)]
    pub oppgitt_aarsak: Aarsak,
}

impl Avsluttet {
    pub fn hendelse_type(&self) -> &'static str {
        AVSLUTTET_HENDELSE_TYPE
    }
}
