use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;

use crate::vo::{Metadata, Opplysning};

pub const AVVIST_STOPP_AV_PERIODE_HENDELSE_TYPE: &str = "intent.v1.avvist_stopp_av_periode";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvvistStoppAvPeriode {
    pub hendelse_id: Uuid,
    pub id: i64,
    pub identitetsnummer: String,
    pub metadata: Metadata,
    #[serde(default)]
    pub opplysninger: HashSet<Opplysning>,
}

impl AvvistStoppAvPeriode {
    pub fn hendelse_type(&self) -> &'static str {
        AVVIST_STOPP_AV_PERIODE_HENDELSE_TYPE
    }
}
