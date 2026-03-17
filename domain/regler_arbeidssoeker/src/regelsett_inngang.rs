use crate::regler::betingelse::Betingelse::{ErNorskEllerTredjelandsborger, Har, HarIkke};
use crate::regler::regel::{Aksjon, Regel};
use crate::regler::regel_id::RegelId;
use crate::regler::regelsett::Regelsett;
use interne_hendelser::vo::Opplysning;

/// inngang differences from utgang:
/// - `EuEoesStatsborgerOver18Aar` additionally requires `!IkkeBosatt`.
/// - Includes the `EuEoesStatsborgerMenHarStatusIkkeBosatt` rule (Arena compatibility).
pub fn regelsett_inngang() -> Regelsett {
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
                    HarIkke(Opplysning::ErNorskStatsborger),
                    HarIkke(Opplysning::IkkeBosatt),
                ],
                Aksjon::GrunnlagForGodkjenning,
            ),
            // Separate rule for EU/EØS citizen with status 'ikke bosatt' (Arena compatibility).
            Regel::new(
                RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt,
                vec![
                    Har(Opplysning::ErEuEoesStatsborger),
                    HarIkke(Opplysning::ErNorskStatsborger),
                    Har(Opplysning::IkkeBosatt),
                ],
                Aksjon::MuligGrunnlagForAvvisning,
            ),
            Regel::new(
                RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
                vec![
                    HarIkke(Opplysning::BosattEtterFregLoven),
                    ErNorskEllerTredjelandsborger,
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

    fn avviste_regel_ider(opplysninger: &[Opplysning]) -> Vec<RegelId> {
        match regelsett_inngang().evaluer(opplysninger) {
            Ok(_) => panic!("Forventet avvisning, men fikk godkjenning"),
            Err(problemer) => problemer.into_iter().map(|p| p.regel_id).collect(),
        }
    }

    fn er_godkjent(opplysninger: &[Opplysning]) -> bool {
        regelsett_inngang().evaluer(opplysninger).is_ok()
    }

    // --- Under 18 år, ikke forhåndsgodkjent ---

    #[test]
    fn under_18_avvises_selv_om_alt_annet_er_ok() {
        let ids = avviste_regel_ider(&[
            ErUnder18Aar,
            BosattEtterFregLoven,
            HarNorskAdresse,
            ErNorskStatsborger,
        ]);
        assert_eq!(ids, vec![RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_avvises_med_ikke_bosatt_nar_ingen_statsborgerskap_info() {
        let mut ids = avviste_regel_ider(&[ErUnder18Aar, IkkeBosatt]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn norsk_statsborger_under_18_avvises_med_ikke_bosatt() {
        let mut ids = avviste_regel_ider(&[ErUnder18Aar, IkkeBosatt, ErNorskStatsborger]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn eu_eoes_under_18_avvises_med_ikke_bosatt_status() {
        let mut ids = avviste_regel_ider(&[ErUnder18Aar, IkkeBosatt, ErEuEoesStatsborger]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn eu_eoes_under_18_med_dnummer_og_ikke_utflyttet_avvises_kun_for_alder() {
        let ids = avviste_regel_ider(&[
            ErUnder18Aar,
            ErEuEoesStatsborger,
            HarUtenlandskAdresse,
            HarRegistrertAdresseIEuEoes,
            IngenInformasjonOmOppholdstillatelse,
            Dnummer,
            IngenFlytteInformasjon,
        ]);
        assert_eq!(ids, vec![RegelId::Under18Aar]);
    }

    #[test]
    fn eu_eoes_under_18_med_dnummer_og_utflyttet_avvises_for_alder_og_ikke_bosatt_status() {
        let mut ids = avviste_regel_ider(&[
            ErUnder18Aar,
            ErEuEoesStatsborger,
            HarUtenlandskAdresse,
            HarRegistrertAdresseIEuEoes,
            IngenInformasjonOmOppholdstillatelse,
            Dnummer,
            IngenFlytteInformasjon,
            IkkeBosatt,
        ]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    // --- Under 18 år, forhåndsgodkjent, skal avvises ---

    #[test]
    fn under_18_forhaandsgodkjent_er_doed_avvises() {
        let mut ids = avviste_regel_ider(&[
            Doed,
            ForhaandsgodkjentAvAnsatt,
            ErUnder18Aar,
            BosattEtterFregLoven,
        ]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![RegelId::Doed, RegelId::Under18Aar];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn under_18_forhaandsgodkjent_er_savnet_avvises() {
        let mut ids = avviste_regel_ider(&[Savnet, ForhaandsgodkjentAvAnsatt, ErUnder18Aar]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Savnet,
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn ikke_funnet_i_pdl_avvises() {
        let ids = avviste_regel_ider(&[PersonIkkeFunnet]);
        assert_eq!(ids, vec![RegelId::IkkeFunnet]);
    }

    // --- Under 18 år, forhåndsgodkjent, skal godkjennes ---

    #[test]
    fn under_18_forhaandsgodkjent_ikke_bosatt_godkjennes() {
        assert!(er_godkjent(&[
            IkkeBosatt,
            ErUnder18Aar,
            ForhaandsgodkjentAvAnsatt
        ]));
    }

    #[test]
    fn under_18_forhaandsgodkjent_ingen_bosatt_info_godkjennes() {
        assert!(er_godkjent(&[ErUnder18Aar, ForhaandsgodkjentAvAnsatt]));
    }

    // --- Over 18 år, skal avvises ---

    #[test]
    fn over_18_norsk_statsborger_ikke_bosatt_avvises() {
        let ids = avviste_regel_ider(&[
            ErNorskStatsborger,
            ErEuEoesStatsborger,
            ErOver18Aar,
            IkkeBosatt,
        ]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }

    #[test]
    fn over_18_tredjelandsborger_ikke_bosatt_avvises() {
        let ids = avviste_regel_ider(&[ErOver18Aar, IkkeBosatt]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }

    #[test]
    fn over_18_eu_eoes_ikke_norsk_med_ikke_bosatt_avvises() {
        let ids = avviste_regel_ider(&[ErOver18Aar, ErEuEoesStatsborger, IkkeBosatt]);
        assert_eq!(ids, vec![RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt]);
    }

    // --- Over 18 år, skal godkjennes ---

    #[test]
    fn over_18_bosatt_godkjennes() {
        assert!(er_godkjent(&[ErOver18Aar, BosattEtterFregLoven]));
    }

    #[test]
    fn over_18_forhaandsgodkjent_godkjennes() {
        assert!(er_godkjent(&[
            ErOver18Aar,
            IkkeBosatt,
            ForhaandsgodkjentAvAnsatt
        ]));
    }

    #[test]
    fn over_18_eu_eoes_ikke_norsk_uten_ikke_bosatt_godkjennes() {
        assert!(er_godkjent(&[ErOver18Aar, ErEuEoesStatsborger]));
    }

    // --- Statsborgerskap ---

    #[test]
    fn gbr_statsborger_over_18_avvises_ikke_bosatt() {
        let ids = avviste_regel_ider(&[ErOver18Aar, ErGbrStatsborger]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }

    #[test]
    fn gbr_statsborger_under_18_avvises() {
        let mut ids = avviste_regel_ider(&[ErUnder18Aar, ErGbrStatsborger]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn tredjelandsborger_under_18_avvises() {
        let mut ids = avviste_regel_ider(&[ErUnder18Aar]);
        ids.sort_by_key(|id| format!("{:?}", id));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(ids, expected);
    }

    #[test]
    fn tredjelandsborger_over_18_avvises_via_standardregel() {
        let ids = avviste_regel_ider(&[]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }
}
