use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use crate::domain::status_logg_entry::StatusLoggEntry;
use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq)]
pub struct Oppgave {
    pub type_: OppgaveType,
    pub status: OppgaveStatus,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
    pub status_logg: Vec<StatusLoggEntry>,
}

impl Oppgave {
    pub fn new(
        type_: OppgaveType,
        status: OppgaveStatus,
        opplysninger: Vec<String>,
        arbeidssoeker_id: i64,
        identitetsnummer: String,
        tidspunkt: DateTime<Utc>,
        status_logg: Vec<StatusLoggEntry>,
    ) -> Self {
        Self {
            type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            tidspunkt,
            status_logg,
        }
    }
}
