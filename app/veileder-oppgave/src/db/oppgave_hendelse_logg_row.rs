use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct OppgaveHendelseLoggRow {
    pub status: String,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct OppgaveHendelseLoggBatchRow {
    pub oppgave_id: i64,
    pub status: String,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}
