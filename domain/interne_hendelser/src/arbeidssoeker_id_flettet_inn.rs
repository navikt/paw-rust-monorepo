use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;

use crate::vo::Metadata;

pub const ARBEIDSSOEKER_ID_FLETTET_INN: &str = "intern.v1.arbeidssoeker_id_flettet_inn";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbeidssoekerIdFlettetInn {
    pub identitetsnummer: String,
    pub id: i64,
    pub hendelse_id: Uuid,
    pub metadata: Metadata,
    pub kilde: Kilde,
}

impl ArbeidssoekerIdFlettetInn {
    pub fn hendelse_type(&self) -> &'static str {
        ARBEIDSSOEKER_ID_FLETTET_INN
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kilde {
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: HashSet<String>,
}
