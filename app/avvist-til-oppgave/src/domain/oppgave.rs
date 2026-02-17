use thiserror::Error;
use chrono::{DateTime, Utc};
use crate::domain::hendelse_logg_entry::{HendelseLoggEntry, HendelseLoggEntryError};
use crate::domain::oppgave_status::{OppgaveStatus, OppgaveStatusParseError};
use crate::domain::oppgave_type::{OppgaveType, OppgaveTypeParseError};

#[derive(Debug, PartialEq)]
pub struct Oppgave {
    pub id: i64,
    pub type_: OppgaveType,
    pub status: OppgaveStatus,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub ekstern_oppgave_id: Option<i64>,
    pub tidspunkt: DateTime<Utc>,
    pub hendelse_logg: Vec<HendelseLoggEntry>,
}

impl Oppgave {
    pub fn new(
        id: i64,
        type_: String,
        status: String,
        opplysninger: Vec<String>,
        arbeidssoeker_id: i64,
        identitetsnummer: String,
        ekstern_oppgave_id: Option<i64>,
        tidspunkt: DateTime<Utc>,
        hendelse_logg: Vec<HendelseLoggEntry>,
    ) -> Result<Self, OppgaveError> {
        Ok(Self {
            id,
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
    #[error(transparent)]
    HendelseLoggEntryError(#[from] HendelseLoggEntryError),
}

#[cfg(test)]
mod tests {
    use crate::domain::oppgave_status::OppgaveStatus;
    use crate::domain::oppgave_type::OppgaveType;
    use super::*;

    #[test]
    fn ny_opg_med_ugyldig_type_kaster_type_parse_error() {
        let ugyldig_type = "Hubba bubba";
        let result = Oppgave::new(
            1,
            ugyldig_type.to_string(),
            OppgaveStatus::Ubehandlet.to_string(),
            vec![],
            12345,
            "12345678901".to_string(),
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
    fn ny_opg_med_ugyldig_status_kaster_type_parse_error() {
        let ugyldig_status = "Bubba hubba";
        let result = Oppgave::new(
            1,
            OppgaveType::AvvistUnder18.to_string(),
            ugyldig_status.to_string(),
            vec![],
            12345,
            "12345678901".to_string(),
            Some(12341),
            Utc::now(),
            vec![],
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            OppgaveError::StatusParseError(e) => {
                assert_eq!(
                    e.to_string(),
                    format!("Ukjent oppgavestatus: {}", ugyldig_status)
                )
            }
            _ => panic!("Forventet StatusParseError"),
        }
    }
}
