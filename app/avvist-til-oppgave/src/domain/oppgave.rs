use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use crate::domain::status_logg_entry::StatusLoggEntry;
use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq)]
pub struct Oppgave {
    pub type_: OppgaveType,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
    pub status_logg: Vec<StatusLoggEntry>,
}

impl Oppgave {
    pub fn new(
        type_: OppgaveType,
        opplysninger: Vec<String>,
        arbeidssoeker_id: i64,
        identitetsnummer: String,
        tidspunkt: DateTime<Utc>,
        status_logg: Vec<StatusLoggEntry>,
    ) -> Self {
        Self {
            type_,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            tidspunkt,
            status_logg,
        }
    }

    pub fn gjeldende_status(&self) -> Option<OppgaveStatus> {
        self.status_logg
            .iter()
            .max_by_key(|entry| entry.tidspunkt)
            .map(|entry| entry.status.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_gjeldende_status() {
        let status_logg: Vec<StatusLoggEntry> = [
            StatusLoggEntry::new(OppgaveStatus::Ubehandlet, Utc::now() - Duration::days(2)),
            StatusLoggEntry::new(OppgaveStatus::Feilet, Utc::now() - Duration::days(1)),
            StatusLoggEntry::new(OppgaveStatus::Ferdigbehandlet, Utc::now()),
        ]
        .to_vec();

        let oppgave: Oppgave = Oppgave::new(
            OppgaveType::AvvistUnder18,
            vec![
                "ER_UNDER_18_AAR".to_string(),
                "BOSATT_ETTER_FREG_LOVEN".to_string(),
            ],
            12345,
            "12345678910".to_string(),
            Utc::now(),
            status_logg,
        );

        assert_eq!(
            oppgave.gjeldende_status().unwrap(),
            OppgaveStatus::Ferdigbehandlet
        );
    }
}
