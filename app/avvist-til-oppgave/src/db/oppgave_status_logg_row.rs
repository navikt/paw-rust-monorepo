use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct OppgaveStatusLoggRow {
    pub oppgave_id: i64,
    pub status: String,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}
