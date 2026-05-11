use thiserror::Error;
use chrono::{DateTime, Utc};
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;
use crate::domain::ekstern_oppgave_id::EksternOppgaveId;
use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
use crate::domain::oppgave_id::OppgaveId;
use crate::domain::oppgave_status::{OppgaveStatus, OppgaveStatusParseError};
use crate::domain::oppgave_type::{OppgaveType, OppgaveTypeParseError};

#[derive(Debug, PartialEq)]
pub struct Oppgave {
    pub id: Option<OppgaveId>,
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
        type_: OppgaveType,
        status: OppgaveStatus,
        opplysninger: Vec<String>,
        arbeidssoeker_id: ArbeidssoekerId,
        identitetsnummer: Identitetsnummer,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id: None,
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
        type_: String,
        status: String,
        opplysninger: Vec<String>,
        arbeidssoeker_id: ArbeidssoekerId,
        identitetsnummer: Identitetsnummer,
        ekstern_oppgave_id: Option<EksternOppgaveId>,
        tidspunkt: DateTime<Utc>,
        hendelse_logg: Vec<HendelseLoggEntry>,
    ) -> Result<Self, OppgaveError> {
        Ok(Self {
            id: Some(id),
            type_: type_.parse()?,
            status: status.parse()?,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            ekstern_oppgave_id,
            tidspunkt,
            hendelse_logg,
        })
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum OppgaveError {
    #[error(transparent)]
    StatusParseError(#[from] OppgaveStatusParseError),
    #[error(transparent)]
    TypeParseError(#[from] OppgaveTypeParseError),
}

#[cfg(test)]
mod tests {
    use crate::domain::oppgave_status::OppgaveStatus;
    use crate::domain::oppgave_type::OppgaveType;
    use types::identitetsnummer::Identitetsnummer;
    use super::*;

    #[test]
    fn rehydrer_med_ugyldig_type_kaster_type_parse_error() {
        let ugyldig_type = "Hubba bubba";
        let result = Oppgave::fra_db(
            OppgaveId(1),
            ugyldig_type.to_string(),
            OppgaveStatus::Ubehandlet.to_string(),
            vec![],
            ArbeidssoekerId(12345),
            Identitetsnummer::new("12345678901".to_string()).unwrap(),
            None,
            Utc::now(),
            vec![],
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            OppgaveError::TypeParseError(e) => {
                assert_eq!(
                    e.to_string(),
                    format!("Ukjent oppgavetype: {}", ugyldig_type)
                )
            }
            _ => panic!("Forventet TypeParseError"),
        }
    }

    #[test]
    fn rehydrer_med_ugyldig_status_kaster_status_parse_error() {
        let ugyldig_status = "Bubba hubba";
        let result = Oppgave::fra_db(
            OppgaveId(1),
            OppgaveType::AvvistUnder18.to_string(),
            ugyldig_status.to_string(),
            vec![],
            ArbeidssoekerId(12345),
            Identitetsnummer::new("12345678901".to_string()).unwrap(),
            Some(EksternOppgaveId(12341)),
            Utc::now(),
            vec![],
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            OppgaveError::StatusParseError(e) => {
                assert_eq!(
                    e.to_string(),
                    format!("Ugyldig oppgavestatus: {}", ugyldig_status)
                )
            }
            _ => panic!("Forventet StatusParseError"),
        }
    }
}
