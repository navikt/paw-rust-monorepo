use crate::domain::kriterier::kriterium::{OppgaveKriterier, Kriterium};
use crate::domain::oppgave_type::OppgaveType;
use interne_hendelser::Startet;
use interne_hendelser::vo::{BrukerType, Opplysning};

pub const KRITERIER: OppgaveKriterier<Startet> = OppgaveKriterier::new(
    OppgaveType::VurderOpphold,
    &[
        Kriterium {
            navn: "innsendt_av_sluttbruker",
            sjekk: |hendelse| hendelse.metadata.utfoert_av.bruker_type == BrukerType::Sluttbruker,
        },
        Kriterium {
            navn: "utflyttet",
            sjekk: |hendelse| hendelse.opplysninger.contains(&Opplysning::IkkeBosatt),
        },
        Kriterium {
            navn: "eu_eoes_statsborger",
            sjekk: |hendelse| hendelse.opplysninger.contains(&Opplysning::ErEuEoesStatsborger),
        },
        Kriterium {
            navn: "ikke_norsk_statsborger",
            sjekk: |hendelse| !hendelse.opplysninger.contains(&Opplysning::ErNorskStatsborger),
        },
    ],
);

#[cfg(test)]
mod tests {
    use super::*;
    use paw_test::hendelse_builder::StartetBuilder;
    use std::collections::HashSet;

    #[test]
    fn oppfylt_naar_utflyttet_eu_eoes_uten_norsk_statsborgerskap() {
        let hendelse = StartetBuilder {
            bruker_type: BrukerType::Sluttbruker,
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
            .build();
        assert!(KRITERIER.oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_uten_opplysninger() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::new(),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_med_kun_norsk_statsborgerskap() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::from([Opplysning::ErNorskStatsborger]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_med_kun_eu_eoes_statsborgerskap() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::from([Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_med_eu_eoes_og_norsk_statsborgerskap() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::from([
                Opplysning::ErEuEoesStatsborger,
                Opplysning::ErNorskStatsborger,
            ]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_naar_kun_utflyttet() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::from([Opplysning::IkkeBosatt]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_naar_utflyttet_med_norsk_statsborgerskap() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::from([
                Opplysning::IkkeBosatt,
                Opplysning::ErNorskStatsborger,
            ]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_naar_utflyttet_eu_eoes_med_norsk_statsborgerskap() {
        let hendelse = StartetBuilder {
            opplysninger: HashSet::from([
                Opplysning::IkkeBosatt,
                Opplysning::ErEuEoesStatsborger,
                Opplysning::ErNorskStatsborger,
            ]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_naar_innsendt_av_veileder() {
        let hendelse = StartetBuilder {
            bruker_type: BrukerType::Veileder,
            utfoert_av_id: "Z991459".to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_naar_innsendt_av_system() {
        let hendelse = StartetBuilder {
            bruker_type: BrukerType::System,
            utfoert_av_id: "Testsystem".to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.ikke_oppfylt_av(&hendelse));
    }
}
