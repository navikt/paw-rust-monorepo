use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSecondsWithFrac};
use std::collections::HashSet;
use uuid::Uuid;

use crate::vo::Metadata;

pub const AUTOMATISK_ID_MERGE_IKKE_MULIG: &str = "intern.v1.automatisk_id_merge_ikke_mulig";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomatiskIdMergeIkkeMulig {
    pub identitetsnummer: String,
    pub id: i64,
    pub hendelse_id: Uuid,
    pub metadata: Metadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gjeldene_identitetsnummer: Option<String>,
    pub pdl_identitetsnummer: HashSet<String>,
    pub lokale_alias: HashSet<Alias>,
    pub perioder: HashSet<PeriodeRad>,
}

impl AutomatiskIdMergeIkkeMulig {
    pub fn hendelse_type(&self) -> &'static str {
        AUTOMATISK_ID_MERGE_IKKE_MULIG
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alias {
    pub identitetsnummer: String,
    pub arbeidsoeker_id: i64,
    pub record_key: i64,
    pub partition: i32,
}
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodeRad {
    pub periode_id: Uuid,
    pub identitetsnummer: String,
    #[serde_as(as = "TimestampSecondsWithFrac<f64>")]
    pub fra: DateTime<Utc>,
    #[serde_as(as = "Option<TimestampSecondsWithFrac<f64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub til: Option<DateTime<Utc>>,
}

impl PeriodeRad {
    pub fn er_aktiv(&self) -> bool {
        self.til.is_none()
    }
}
