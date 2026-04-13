use crate::regler::betingelse::Betingelse::{ErTredjelandsborger, Har, HarIkke};
use crate::regler::regel::{Aksjon, Regel};
use crate::regler::regel_id::RegelId;
use crate::regler::regelsett::Regelsett;
use interne_hendelser::vo::Opplysning;
//Oppdatert regelsett versjon 3.
//Alle brukere fra Eu/Eøs som er over 18 år godkjennes nå uavhengig av bosettingsstatus,
//norske statusborgere behandles nå på samme måte som andre Eu/Eøs.
//Det er nå bare tredjelandsborgere som kan avvises på grunn av bosettingsstatus.
pub fn regelsett_v4() -> Regelsett {
    Regelsett {
        regler: vec![
            Regel::new(
                RegelId::IkkeFunnet,
                vec![Har(Opplysning::PersonIkkeFunnet)],
                Aksjon::SkalAvvises,
            ),
            Regel::new(
                RegelId::Doed,
                vec![Har(Opplysning::Doed)],
                Aksjon::SkalAvvises,
            ),
            Regel::new(
                RegelId::Savnet,
                vec![Har(Opplysning::Savnet)],
                Aksjon::SkalAvvises,
            ),
            Regel::new(
                RegelId::Opphoert,
                vec![Har(Opplysning::OpphoertIdentitet)],
                Aksjon::SkalAvvises,
            ),
            Regel::new(
                RegelId::ForhaandsgodkjentAvAnsatt,
                vec![Har(Opplysning::ForhaandsgodkjentAvAnsatt)],
                Aksjon::GrunnlagForGodkjenning,
            ),
            Regel::new(
                RegelId::Under18Aar,
                vec![Har(Opplysning::ErUnder18Aar)],
                Aksjon::MuligGrunnlagForAvvisning,
            ),
            Regel::new(
                RegelId::UkjentAlder,
                vec![
                    Har(Opplysning::UkjentFoedselsaar),
                    Har(Opplysning::UkjentFoedselsdato),
                ],
                Aksjon::MuligGrunnlagForAvvisning,
            ),
            Regel::new(
                RegelId::Over18AarOgBosattEtterFregLoven,
                vec![
                    Har(Opplysning::ErOver18Aar),
                    Har(Opplysning::BosattEtterFregLoven),
                ],
                Aksjon::GrunnlagForGodkjenning,
            ),
            Regel::new(
                RegelId::EuEoesStatsborgerOver18Aar,
                vec![
                    Har(Opplysning::ErOver18Aar),
                    Har(Opplysning::ErEuEoesStatsborger),
                ],
                Aksjon::GrunnlagForGodkjenning,
            ),
            Regel::new(
                RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
                vec![
                    HarIkke(Opplysning::BosattEtterFregLoven),
                    ErTredjelandsborger,
                ],
                Aksjon::MuligGrunnlagForAvvisning,
            ),
        ],
        standard_regel: Regel::new(
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
            vec![],
            Aksjon::MuligGrunnlagForAvvisning,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interne_hendelser::vo::Opplysning::*;
    use std::collections::HashSet;

    struct Evaluering<'a>(&'a [Opplysning]);

    fn gitt(opplysninger: &[Opplysning]) -> Evaluering<'_> {
        Evaluering(opplysninger)
    }

    impl Evaluering<'_> {
        fn skal_godkjennes(self) {
            assert!(
                regelsett_v4().evaluer(self.0).is_ok(),
                "Forventet godkjenning, men fikk avvisning"
            );
        }

        fn skal_avvises_med(self, forventet: &[RegelId]) {
            let ids: HashSet<_> = match regelsett_v4().evaluer(self.0) {
                Ok(_) => panic!("Forventet avvisning, men fikk godkjenning"),
                Err(problemer) => problemer.into_iter().map(|p| p.regel_id).collect(),
            };
            let forventet: HashSet<_> = forventet.iter().cloned().collect();
            assert_eq!(ids, forventet);
        }
    }

    // --- Alltid avvist ---

    #[test]
    fn person_ikke_funnet_avvises() {
        gitt(&[PersonIkkeFunnet]).skal_avvises_med(&[RegelId::IkkeFunnet]);
    }

    #[test]
    fn doed_person_avvises() {
        gitt(&[Doed, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar])
            .skal_avvises_med(&[RegelId::Doed]);
    }

    #[test]
    fn savnet_person_avvises() {
        gitt(&[Savnet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar])
            .skal_avvises_med(&[RegelId::Savnet]);
    }

    #[test]
    fn opphoert_identitet_avvises() {
        gitt(&[OpphoertIdentitet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar])
            .skal_avvises_med(&[RegelId::Opphoert]);
    }

    #[test]
    fn forhaandsgodkjenning_overstyrer_ikke_doed() {
        gitt(&[Doed, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar])
            .skal_avvises_med(&[RegelId::Doed]);
    }

    #[test]
    fn forhaandsgodkjenning_overstyrer_ikke_savnet() {
        gitt(&[Savnet, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar])
            .skal_avvises_med(&[RegelId::Savnet]);
    }

    #[test]
    fn forhaandsgodkjenning_overstyrer_ikke_opphoert() {
        gitt(&[OpphoertIdentitet, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar])
            .skal_avvises_med(&[RegelId::Opphoert]);
    }

    // --- Under 18 år ---

    #[test]
    fn eu_eoes_under_18_bosatt_avvises_kun_for_alder() {
        gitt(&[ErUnder18Aar, BosattEtterFregLoven, ErNorskStatsborger, ErEuEoesStatsborger])
            .skal_avvises_med(&[RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_tredjelandsborger_uten_bosatt_avvises_for_alder_og_bosatt() {
        gitt(&[ErUnder18Aar]).skal_avvises_med(&[
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ]);
    }

    #[test]
    fn under_18_eu_eoes_uten_bosatt_avvises_kun_for_alder() {
        gitt(&[ErUnder18Aar, ErEuEoesStatsborger]).skal_avvises_med(&[RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_norsk_uten_bosatt_avvises_kun_for_alder() {
        gitt(&[ErUnder18Aar, ErNorskStatsborger, ErEuEoesStatsborger])
            .skal_avvises_med(&[RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_forhaandsgodkjent_godkjennes() {
        gitt(&[ErUnder18Aar, ForhaandsgodkjentAvAnsatt]).skal_godkjennes();
    }

    #[test]
    fn under_18_forhaandsgodkjent_doed_avvises() {
        gitt(&[Doed, ForhaandsgodkjentAvAnsatt, ErUnder18Aar, BosattEtterFregLoven])
            .skal_avvises_med(&[RegelId::Doed, RegelId::Under18Aar]);
    }

    // --- Ukjent alder ---

    #[test]
    fn ukjent_alder_tredjelandsborger_avvises_for_alder_og_bosatt() {
        gitt(&[UkjentFoedselsaar, UkjentFoedselsdato]).skal_avvises_med(&[
            RegelId::UkjentAlder,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ]);
    }

    #[test]
    fn ukjent_alder_eu_eoes_avvises_kun_for_alder() {
        gitt(&[UkjentFoedselsaar, UkjentFoedselsdato, ErEuEoesStatsborger])
            .skal_avvises_med(&[RegelId::UkjentAlder]);
    }

    // --- Over 18, godkjennes ---

    #[test]
    fn over_18_bosatt_godkjennes() {
        gitt(&[ErOver18Aar, BosattEtterFregLoven]).skal_godkjennes();
    }

    #[test]
    fn over_18_eu_eoes_uten_bosatt_godkjennes() {
        gitt(&[ErOver18Aar, ErEuEoesStatsborger]).skal_godkjennes();
    }

    #[test]
    fn over_18_norsk_uten_bosatt_godkjennes() {
        gitt(&[ErOver18Aar, ErNorskStatsborger, ErEuEoesStatsborger]).skal_godkjennes();
    }

    #[test]
    fn over_18_forhaandsgodkjent_uten_bosatt_godkjennes() {
        gitt(&[ErOver18Aar, ForhaandsgodkjentAvAnsatt]).skal_godkjennes();
    }

    // --- Over 18, avvises ---

    #[test]
    fn over_18_tredjelandsborger_uten_bosatt_avvises() {
        gitt(&[ErOver18Aar])
            .skal_avvises_med(&[RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]);
    }

    #[test]
    fn over_18_gbr_statsborger_uten_bosatt_avvises() {
        gitt(&[ErOver18Aar, ErGbrStatsborger])
            .skal_avvises_med(&[RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]);
    }

    // --- Standardregel ---

    #[test]
    fn norsk_eu_eoes_uten_aldersinfo_avvises_via_standardregel() {
        gitt(&[ErNorskStatsborger, ErEuEoesStatsborger])
            .skal_avvises_med(&[RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]);
    }
}
