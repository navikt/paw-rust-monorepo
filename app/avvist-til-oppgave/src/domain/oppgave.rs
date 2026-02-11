use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq)]
pub struct Oppgave {
    pub id: i64,
    pub type_: OppgaveType,
    pub status: OppgaveStatus,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
    pub hendelse_logg: Vec<HendelseLoggEntry>,
}

impl Oppgave {
    pub fn new(
        id: i64,
        type_: OppgaveType,
        status: OppgaveStatus,
        opplysninger: Vec<String>,
        arbeidssoeker_id: i64,
        identitetsnummer: String,
        tidspunkt: DateTime<Utc>,
        hendelse_logg: Vec<HendelseLoggEntry>,
    ) -> Self {
        Self {
            id,
            type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            tidspunkt,
            hendelse_logg,
        }
    }
}
