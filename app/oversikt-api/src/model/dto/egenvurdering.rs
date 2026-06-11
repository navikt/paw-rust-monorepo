use crate::model::dto::profilering::ProfilertTil;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Egenvurdering {
    pub id: Uuid,
    pub egenvurdert_til: ProfilertTil,
    pub tidspunkt: DateTime<Utc>,
}
