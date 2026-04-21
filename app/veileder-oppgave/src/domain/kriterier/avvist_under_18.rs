use interne_hendelser::Avvist;
use interne_hendelser::vo::{BrukerType, Opplysning};
use crate::domain::kriterier::kriterium::{Kriterier, Kriterium};

pub const KRITERIER: Kriterier<Avvist> = Kriterier::new(&[
    Kriterium {
        navn: "innsendt_av_sluttbruker",
        sjekk: |hendelse| hendelse.metadata.utfoert_av.bruker_type == BrukerType::Sluttbruker,
    },
    Kriterium {
        navn: "er_under_18",
        sjekk: |hendelse| hendelse.opplysninger.contains(&Opplysning::ErUnder18Aar),
    },
]);

#[cfg(test)]
mod tests {
    use super::*;
    use paw_test::hendelse_builder::AvvistBuilder;
    use std::collections::HashSet;

    #[test]
    fn oppfylt_for_sluttbruker_under_18() {
        let hendelse = AvvistBuilder {
            bruker_type: BrukerType::Sluttbruker,
            opplysninger: HashSet::from([Opplysning::ErUnder18Aar]),
            ..Default::default()
        }
        .build();
        assert!(KRITERIER.oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_for_sluttbruker_over_18() {
        let hendelse = AvvistBuilder {
            bruker_type: BrukerType::Sluttbruker,
            opplysninger: HashSet::from([Opplysning::ErOver18Aar]),
            ..Default::default()
        }
        .build();
        assert!(!KRITERIER.oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_for_system_under_18() {
        let hendelse = AvvistBuilder {
            bruker_type: BrukerType::System,
            utfoert_av_id: "Testsystem".to_string(),
            opplysninger: HashSet::from([Opplysning::ErUnder18Aar]),
            ..Default::default()
        }
        .build();
        assert!(!KRITERIER.oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_for_system_over_18() {
        let hendelse = AvvistBuilder {
            bruker_type: BrukerType::System,
            utfoert_av_id: "Testsystem".to_string(),
            opplysninger: HashSet::from([Opplysning::ErOver18Aar]),
            ..Default::default()
        }
        .build();
        assert!(!KRITERIER.oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_for_veileder_under_18() {
        let hendelse = AvvistBuilder {
            bruker_type: BrukerType::Veileder,
            utfoert_av_id: "Z991459".to_string(),
            opplysninger: HashSet::from([Opplysning::ErUnder18Aar]),
            ..Default::default()
        }
        .build();
        assert!(!KRITERIER.oppfylt_av(&hendelse));
    }

    #[test]
    fn ikke_oppfylt_for_veileder_over_18() {
        let hendelse = AvvistBuilder {
            bruker_type: BrukerType::Veileder,
            utfoert_av_id: "Z991459".to_string(),
            opplysninger: HashSet::from([Opplysning::ErOver18Aar]),
            ..Default::default()
        }
        .build();
        assert!(!KRITERIER.oppfylt_av(&hendelse));
    }
}
