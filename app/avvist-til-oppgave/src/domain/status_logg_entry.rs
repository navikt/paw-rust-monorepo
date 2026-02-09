use crate::domain::oppgave_status::OppgaveStatus;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct StatusLoggEntry {
    pub status: OppgaveStatus,
    pub tidspunkt: DateTime<Utc>,
}

impl StatusLoggEntry {
    pub fn new(status: OppgaveStatus, tidspunkt: DateTime<Utc>) -> Self {
        Self { status, tidspunkt }
    }
}
