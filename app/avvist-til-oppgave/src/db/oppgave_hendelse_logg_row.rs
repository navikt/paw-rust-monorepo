use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug)]
pub struct InsertOppgaveHendelseLoggRow {
    pub oppgave_id: i64,
    pub status: String,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct OppgaveHendelseLoggRow {
    pub id: i64,
    pub oppgave_id: i64,
    pub status: String,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}
