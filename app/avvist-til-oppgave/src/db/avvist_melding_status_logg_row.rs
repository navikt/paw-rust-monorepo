use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AvvistMeldingStatusLoggRow {
    pub melding_id: Uuid,
    pub status: String,
    pub tidspunkt: f64,
}
