use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::{DateTime, Utc};
use interne_hendelser::Hendelse;
use sqlx::FromRow;
use uuid::Uuid;
use crate::domain::arbeidssoeker_id::ArbeidssoekerId;

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
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
}

pub fn to_oppgave_row(
    hendelse: &impl Hendelse,
    oppgave_type: OppgaveType,
    oppgave_status: OppgaveStatus,
) -> InsertOppgaveRow {
    let opplysninger: Vec<String> = hendelse
        .opplysninger()
        .iter()
        .map(|opplysning| opplysning.to_string())
        .collect();

    InsertOppgaveRow {
        type_: oppgave_type.to_string(),
        status: oppgave_status.to_string(),
        melding_id: hendelse.hendelse_id(),
        opplysninger,
        arbeidssoeker_id: ArbeidssoekerId::from(hendelse.id()),
        identitetsnummer: hendelse.identitetsnummer().to_string(),
        tidspunkt: hendelse.metadata().tidspunkt,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use interne_hendelser::vo::Opplysning;
    use paw_test::hendelse_builder::{AvvistBuilder, StartetBuilder};
    use std::collections::HashSet;

    #[test]
    fn test_avvist_hendelse_to_oppgave_row() {
        let avvist_hendelse = AvvistBuilder {
            arbeidssoeker_id: 12345,
            identitetsnummer: "12345678901".to_string(),
            opplysninger: HashSet::from([Opplysning::ErUnder18Aar, Opplysning::BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();

        let oppgave_row = to_oppgave_row(
            &avvist_hendelse,
            OppgaveType::AvvistUnder18,
            OppgaveStatus::Ubehandlet,
        );

        assert_eq!(oppgave_row.melding_id, avvist_hendelse.hendelse_id);
        assert_eq!(oppgave_row.type_, OppgaveType::AvvistUnder18.to_string());
        assert_eq!(oppgave_row.status, OppgaveStatus::Ubehandlet.to_string());
        let forventede: HashSet<String> = avvist_hendelse.opplysninger.iter().map(|o| o.to_string()).collect();
        assert_eq!(oppgave_row.opplysninger.into_iter().collect::<HashSet<_>>(), forventede);
        assert_eq!(oppgave_row.identitetsnummer, avvist_hendelse.identitetsnummer);
        assert_eq!(oppgave_row.arbeidssoeker_id, ArbeidssoekerId::from(avvist_hendelse.id));
        assert_eq!(oppgave_row.tidspunkt, avvist_hendelse.metadata.tidspunkt);
    }

    #[test]
    fn test_startet_hendelse_to_oppgave_row() {
        let startet_hendelse = StartetBuilder {
            arbeidssoeker_id: 67890,
            identitetsnummer: "98765432109".to_string(),
            opplysninger: HashSet::from([Opplysning::ErEuEoesStatsborger, Opplysning::IkkeBosatt]),
            ..Default::default()
        }
        .build();

        let oppgave_row = to_oppgave_row(
            &startet_hendelse,
            OppgaveType::VurderOppholdsstatus,
            OppgaveStatus::Ubehandlet,
        );

        assert_eq!(oppgave_row.melding_id, startet_hendelse.hendelse_id);
        assert_eq!(oppgave_row.type_, OppgaveType::VurderOppholdsstatus.to_string());
        assert_eq!(oppgave_row.status, OppgaveStatus::Ubehandlet.to_string());
        let forventede_opplysninger: HashSet<String> = startet_hendelse.opplysninger.iter().map(|opplysning| opplysning.to_string()).collect();
        assert_eq!(oppgave_row.opplysninger.into_iter().collect::<HashSet<_>>(), forventede_opplysninger);
        assert_eq!(oppgave_row.identitetsnummer, startet_hendelse.identitetsnummer);
        assert_eq!(oppgave_row.arbeidssoeker_id, ArbeidssoekerId::from(startet_hendelse.id));
        assert_eq!(oppgave_row.tidspunkt, startet_hendelse.metadata.tidspunkt);
    }
}
