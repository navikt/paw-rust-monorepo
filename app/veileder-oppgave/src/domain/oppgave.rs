use chrono::{DateTime, Utc};
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;
use uuid::Uuid;
use crate::domain::ekstern_oppgave_id::EksternOppgaveId;
use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
use crate::domain::oppgave_id::OppgaveId;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;

#[derive(Debug, PartialEq)]
pub struct Oppgave {
    pub id: Option<OppgaveId>,
    pub melding_id: Uuid,
    pub type_: OppgaveType,
    pub status: OppgaveStatus,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: ArbeidssoekerId,
    pub identitetsnummer: Identitetsnummer,
    pub ekstern_oppgave_id: Option<EksternOppgaveId>,
    pub tidspunkt: DateTime<Utc>,
    pub hendelse_logg: Vec<HendelseLoggEntry>,
}

impl Oppgave {
    pub fn id(&self) -> OppgaveId {
        self.id.expect("Oppgave mangler id — ikke persistert")
    }

    pub fn new(
        melding_id: Uuid,
        type_: OppgaveType,
        status: OppgaveStatus,
        opplysninger: Vec<String>,
        arbeidssoeker_id: ArbeidssoekerId,
        identitetsnummer: Identitetsnummer,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id: None,
            melding_id,
            type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            ekstern_oppgave_id: None,
            tidspunkt,
            hendelse_logg: Vec::new(),
        }
    }

    pub fn fra_db(
        id: OppgaveId,
        melding_id: Uuid,
        type_: OppgaveType,
        status: OppgaveStatus,
        opplysninger: Vec<String>,
        arbeidssoeker_id: ArbeidssoekerId,
        identitetsnummer: Identitetsnummer,
        ekstern_oppgave_id: Option<EksternOppgaveId>,
        tidspunkt: DateTime<Utc>,
        hendelse_logg: Vec<HendelseLoggEntry>,
    ) -> Self {
        Self {
            id: Some(id),
            melding_id,
            type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            ekstern_oppgave_id,
            tidspunkt,
            hendelse_logg,
        }
    }
}
