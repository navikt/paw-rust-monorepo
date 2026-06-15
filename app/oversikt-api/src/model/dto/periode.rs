use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Periode {
    pub id: Uuid,
    pub startet: DateTime<Utc>,
    pub avsluttet: Option<DateTime<Utc>>,
}
