use crate::model::dto::bekreftelse::Bekreftelsesloesning;
use crate::model::dto::profilering::ProfilertTil;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedighetsperiodeV2 {
    pub periode_id: Uuid,
    pub ledig_siden: Option<DateTime<Utc>>,
    pub periode_startet: DateTime<Utc>,
    pub egenvurdert_til: Option<ProfilertTil>,
    pub bekreftelse_har_jobbet: Option<bool>,
    pub bekreftelse_vil_fortsette: Option<bool>,
    pub bekreftelse_ansvar: Bekreftelsesloesning,
}
