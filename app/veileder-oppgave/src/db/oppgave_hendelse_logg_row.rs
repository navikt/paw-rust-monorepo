use chrono::{DateTime, Utc};
use sqlx::FromRow;
use crate::domain::oppgave_id::OppgaveId;

#[derive(Debug)]
pub struct InsertOppgaveHendelseLoggRow {
    pub oppgave_id: OppgaveId,
    pub status: String,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct OppgaveHendelseLoggRow {
    pub status: String,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(FromRow)]
pub struct OppgaveHendelseLoggBatchRow {
    pub oppgave_id: i64,
    pub status: String,
    pub tidspunkt: DateTime<Utc>,
}
