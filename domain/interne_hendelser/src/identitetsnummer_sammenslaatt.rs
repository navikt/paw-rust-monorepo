use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;

use crate::vo::Metadata;

pub const IDENTITETSNUMMER_SAMMENSLAATT_HENDELSE_TYPE: &str = "intern.v1.identitetsnummer_sammenslaatt";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentitetsnummerSammenslaatt {
    pub id: i64,
    pub hendelse_id: Uuid,
    pub identitetsnummer: String,
    pub metadata: Metadata,
    pub flyttede_identitetsnumre: HashSet<String>,
    pub flyttet_til_arbeidssoeker_id: i64,
}

impl IdentitetsnummerSammenslaatt {
    pub fn hendelse_type(&self) -> &'static str {
        IDENTITETSNUMMER_SAMMENSLAATT_HENDELSE_TYPE
    }
}
