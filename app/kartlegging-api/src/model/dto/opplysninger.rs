use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opplysninger {
    pub id: Uuid,
    pub tidspunkt: DateTime<Utc>,
}
