use crate::domain::oppgave::Oppgave;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;

#[derive(Debug, FromRow)]
pub struct OppgaveRow {
    pub id: i64,
    pub type_: String,
    pub status: String,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub ekstern_oppgave_id: Option<i64>,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug)]
pub struct InsertOppgaveRow {
    pub type_: String,
    pub status: String,
    pub melding_id: Uuid,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: ArbeidssoekerId,
    pub identitetsnummer: Identitetsnummer,
    pub tidspunkt: DateTime<Utc>,
}

pub fn to_oppgave_insert_row(oppgave: &Oppgave, melding_id: Uuid) -> InsertOppgaveRow {
    InsertOppgaveRow {
        type_: oppgave.type_.to_string(),
        status: oppgave.status.to_string(),
        melding_id,
        opplysninger: oppgave.opplysninger.clone(),
        arbeidssoeker_id: oppgave.arbeidssoeker_id,
        identitetsnummer: oppgave.identitetsnummer.clone(),
        tidspunkt: oppgave.tidspunkt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::oppgave_status::OppgaveStatus;
    use crate::domain::oppgave_type::OppgaveType;
    use chrono::Utc;

    #[test]
    fn test_oppgave_til_insert_row() {
        let melding_id = Uuid::new_v4();
        let tidspunkt = Utc::now();
        let opplysninger = vec!["ER_UNDER_18_AAR".to_string(), "BOSATT_ETTER_FREG_LOVEN".to_string()];
        let arbeidssoeker_id = ArbeidssoekerId(12345);
        let identitetsnummer = Identitetsnummer::new("12345678901".to_string()).unwrap();

        let oppgave = Oppgave::new(
            OppgaveType::VurderOppholdsstatus,
            OppgaveStatus::Ubehandlet,
            opplysninger.clone(),
            arbeidssoeker_id,
            identitetsnummer.clone(),
            tidspunkt,
        );

        let row = to_oppgave_insert_row(&oppgave, melding_id);

        assert_eq!(row.melding_id, melding_id);
        assert_eq!(row.type_, OppgaveType::VurderOppholdsstatus.to_string());
        assert_eq!(row.status, OppgaveStatus::Ubehandlet.to_string());
        assert_eq!(row.opplysninger, opplysninger);
        assert_eq!(row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(row.identitetsnummer, identitetsnummer);
        assert_eq!(row.tidspunkt, tidspunkt);
    }
}
