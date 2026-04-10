use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::{DateTime, Utc};
use interne_hendelser::Avvist;
use interne_hendelser::Startet;
use sqlx::FromRow;
use uuid::Uuid;

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
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
}

pub fn to_oppgave_row(
    avvist: Avvist,
    oppgave_type: OppgaveType,
    oppgave_status: OppgaveStatus,
) -> InsertOppgaveRow {
    let opplysninger: Vec<String> = avvist
        .opplysninger
        .iter()
        .map(|opplysning| opplysning.to_string())
        .collect();

    InsertOppgaveRow {
        type_: oppgave_type.to_string(),
        status: oppgave_status.to_string(),
        melding_id: avvist.hendelse_id,
        opplysninger,
        arbeidssoeker_id: avvist.id,
        identitetsnummer: avvist.identitetsnummer,
        tidspunkt: avvist.metadata.tidspunkt,
    }
}

pub fn startet_to_oppgave_row(
    startet: Startet,
    oppgave_type: OppgaveType,
    oppgave_status: OppgaveStatus,
) -> InsertOppgaveRow {
    let opplysninger: Vec<String> = startet
        .opplysninger
        .iter()
        .map(|opplysning| opplysning.to_string())
        .collect();

    InsertOppgaveRow {
        type_: oppgave_type.to_string(),
        status: oppgave_status.to_string(),
        melding_id: startet.hendelse_id,
        opplysninger,
        arbeidssoeker_id: startet.id,
        identitetsnummer: startet.identitetsnummer,
        tidspunkt: startet.metadata.tidspunkt,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use interne_hendelser::vo::{Bruker, BrukerType, Metadata, Opplysning};
    use paw_rust_base::convenience_functions::contains_all;
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn test_avvist_hendelse_to_oppgave_row() {
        let hendelse_id = Uuid::new_v4();
        let id = 12345;
        let identitetsnummer = "12345678901".to_string();
        let now = Utc::now();

        let mut opplysninger = HashSet::new();
        opplysninger.insert(Opplysning::ErUnder18Aar);
        opplysninger.insert(Opplysning::BosattEtterFregLoven);

        let avvist_hendelse = Avvist {
            hendelse_id,
            id,
            identitetsnummer: identitetsnummer.clone(),
            metadata: Metadata {
                tidspunkt: now,
                utfoert_av: Bruker {
                    bruker_type: BrukerType::System,
                    id: "123".to_string(),
                    sikkerhetsnivaa: None,
                },
                kilde: "Testkilde".to_string(),
                aarsak: "Test årsak".to_string(),
                tidspunkt_fra_kilde: None,
            },
            opplysninger: opplysninger.clone(),
            handling: None,
        };

        let oppgave_row = to_oppgave_row(
            avvist_hendelse.clone(),
            OppgaveType::AvvistUnder18,
            OppgaveStatus::Ubehandlet,
        );

        assert_eq!(oppgave_row.melding_id, avvist_hendelse.hendelse_id);

        assert!(
            contains_all(
                &oppgave_row.opplysninger,
                &[
                    "ER_UNDER_18_AAR".to_string(),
                    "BOSATT_ETTER_FREG_LOVEN".to_string()
                ]
            ),
            "Mangler forventede opplysninger: {:?}",
            oppgave_row.opplysninger
        );
        assert_eq!(
            oppgave_row.identitetsnummer,
            avvist_hendelse.identitetsnummer
        );
        assert_eq!(oppgave_row.arbeidssoeker_id, avvist_hendelse.id);
        assert_eq!(oppgave_row.tidspunkt, avvist_hendelse.metadata.tidspunkt);
    }

    #[test]
    fn test_startet_hendelse_to_oppgave_row() {
        let hendelse_id = Uuid::new_v4();
        let id = 67890;
        let identitetsnummer = "98765432109".to_string();
        let now = Utc::now();

        let mut opplysninger = HashSet::new();
        opplysninger.insert(Opplysning::ErEuEoesStatsborger);
        opplysninger.insert(Opplysning::IkkeBosatt);

        let startet_hendelse = Startet {
            hendelse_id,
            id,
            identitetsnummer: identitetsnummer.clone(),
            metadata: Metadata {
                tidspunkt: now,
                utfoert_av: Bruker {
                    bruker_type: BrukerType::Sluttbruker,
                    id: "456".to_string(),
                    sikkerhetsnivaa: None,
                },
                kilde: "Testkilde".to_string(),
                aarsak: "Test årsak".to_string(),
                tidspunkt_fra_kilde: None,
            },
            opplysninger: opplysninger.clone(),
        };

        let oppgave_row = startet_to_oppgave_row(
            startet_hendelse.clone(),
            OppgaveType::StartetEuEoesIkkeBosatt,
            OppgaveStatus::Ubehandlet,
        );

        assert_eq!(oppgave_row.melding_id, startet_hendelse.hendelse_id);
        assert_eq!(oppgave_row.type_, "STARTET_EU_EOES_IKKE_BOSATT");
        assert!(
            contains_all(
                &oppgave_row.opplysninger,
                &[
                    "ER_EU_EOES_STATSBORGER".to_string(),
                    "IKKE_BOSATT".to_string()
                ]
            ),
            "Mangler forventede opplysninger: {:?}",
            oppgave_row.opplysninger
        );
        assert_eq!(
            oppgave_row.identitetsnummer,
            startet_hendelse.identitetsnummer
        );
        assert_eq!(oppgave_row.arbeidssoeker_id, startet_hendelse.id);
        assert_eq!(oppgave_row.tidspunkt, startet_hendelse.metadata.tidspunkt);
    }
}
