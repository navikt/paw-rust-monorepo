use crate::avvist_hendelse::AvvistHendelse;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct OppgaveRow {
    pub id: i64,
    pub type_: String,
    pub status: String,
    pub melding_id: Uuid,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug)]
pub struct InsertOppgaveRow {
    pub type_: String,
    pub status: String,
    pub melding_id: Uuid,
    pub opplysninger: Vec<String>,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
}

pub fn to_oppgave_row(
    hendelse: AvvistHendelse,
    oppgave_type: OppgaveType,
    oppgave_status: OppgaveStatus,
) -> InsertOppgaveRow {
    InsertOppgaveRow {
        type_: oppgave_type.to_string(),
        status: oppgave_status.to_string(),
        melding_id: hendelse.hendelse_id,
        opplysninger: hendelse.opplysninger,
        arbeidssoeker_id: hendelse.id,
        identitetsnummer: hendelse.identitetsnummer,
        tidspunkt: hendelse.metadata.tidspunkt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avvist_hendelse::{Metadata, UtfoertAv};
    use uuid::Uuid;

    #[test]
    fn test_avvist_hendelse_to_oppgave_row() {
        let hendelse_id = Uuid::new_v4();
        let id = 12345;
        let aarsak = "Test årsak".to_string();
        let identitetsnummer = "12345678901".to_string();
        let now = Utc::now();

        let avvist_hendelse = AvvistHendelse {
            hendelse_id,
            id,
            identitetsnummer: identitetsnummer.clone(),
            metadata: Metadata {
                tidspunkt: now,
                utfoert_av: UtfoertAv {
                    bruker_type: "System".to_string(),
                    id: "123".to_string(),
                },
                kilde: "Testkilde".to_string(),
                aarsak: aarsak.clone(),
            },
            hendelse_type: "TestType".to_string(),
            opplysninger: vec![
                "ER_UNDER_18_AAR".to_string(),
                "BOSATT_ETTER_FREG_LOVEN".to_string(),
            ],
        };

        let oppgave_row = to_oppgave_row(
            avvist_hendelse.clone(),
            OppgaveType::AvvistUnder18,
            OppgaveStatus::Ubehandlet
        );

        assert_eq!(oppgave_row.melding_id, avvist_hendelse.hendelse_id);
        assert_eq!(oppgave_row.opplysninger, avvist_hendelse.opplysninger);
        assert_eq!(
            oppgave_row.identitetsnummer,
            avvist_hendelse.identitetsnummer
        );
        assert_eq!(oppgave_row.arbeidssoeker_id, avvist_hendelse.id);
        assert_eq!(oppgave_row.tidspunkt, avvist_hendelse.metadata.tidspunkt);
    }
}
