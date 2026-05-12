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
    use interne_hendelser::vo::Opplysning;
    use paw_test::hendelse_builder::StartetBuilder;
    use std::collections::HashSet;

    #[test]
    fn test_oppgave_til_insert_row() {
        let startet_hendelse = StartetBuilder {
            arbeidssoeker_id: 67890,
            identitetsnummer: "98765432109".to_string(),
            opplysninger: HashSet::from([Opplysning::ErEuEoesStatsborger, Opplysning::IkkeBosatt]),
            ..Default::default()
        }
        .build();

        let opplysninger: Vec<String> = startet_hendelse
            .opplysninger
            .iter()
            .map(|o| o.to_string())
            .collect();

        let oppgave = Oppgave::new(
            OppgaveType::VurderOppholdsstatus,
            OppgaveStatus::Ubehandlet,
            opplysninger.clone(),
            ArbeidssoekerId::from(startet_hendelse.id),
            Identitetsnummer::new(startet_hendelse.identitetsnummer.clone())
                .expect("Ugyldig identitetsnummer"),
            startet_hendelse.metadata.tidspunkt,
        );

        let row = to_oppgave_insert_row(&oppgave, startet_hendelse.hendelse_id);

        assert_eq!(row.melding_id, startet_hendelse.hendelse_id);
        assert_eq!(row.type_, OppgaveType::VurderOppholdsstatus.to_string());
        assert_eq!(row.status, OppgaveStatus::Ubehandlet.to_string());
        let forventede: HashSet<String> = opplysninger.into_iter().collect();
        assert_eq!(row.opplysninger.into_iter().collect::<HashSet<_>>(), forventede);
        assert_eq!(String::from(row.identitetsnummer), startet_hendelse.identitetsnummer);
        assert_eq!(row.arbeidssoeker_id, ArbeidssoekerId::from(startet_hendelse.id));
        assert_eq!(row.tidspunkt, startet_hendelse.metadata.tidspunkt);
    }
}
