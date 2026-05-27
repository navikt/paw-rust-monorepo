use crate::regler::betingelse::Betingelse::{ErNorskEllerTredjelandsborger, Har, HarIkke};
use crate::regler::regel::{Aksjon, Regel};
use crate::regler::regel_id::RegelId;
use crate::regler::regelsett::Regelsett;
use interne_hendelser::vo::Opplysning;

/// Regelsett versjon 1.
///
/// Alltid avvist: person ikke funnet, død, savnet eller opphørt identitet.
///
/// Grunnlag for godkjenning:
/// - Forhåndsgodkjent av ansatt.
/// - Over 18 år og bosatt etter folkeregisterloven.
/// - EU/EØS-statsborger over 18 år, ikke norsk, og ikke registrert som ikke-bosatt.
///
/// Mulig grunnlag for avvisning:
/// - Under 18 år eller ukjent alder.
/// - EU/EØS-statsborger med status «ikke bosatt» (Arena-kompatibilitetsregel).
/// - Ikke bosatt i Norge etter folkeregisterloven (norsk eller tredjelandsborger).
///
/// Standardregel (fallback): avvises som ikke bosatt.
pub fn regelsett_v2() -> Regelsett {
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
    use crate::regler::regelsett::EvalueringsResultat;
    use interne_hendelser::vo::Opplysning::*;

    fn krever_manuel_vurdering_ider(opplysninger: &[Opplysning]) -> Vec<RegelId> {
        match regelsett_v2().evaluer(opplysninger) {
            EvalueringsResultat::Godkjent { .. } => {
                panic!("Forventet avvisning, men fikk godkjenning")
            }
            EvalueringsResultat::Avvist { regel_ider } => regel_ider,
            EvalueringsResultat::KreverManuellVurdering { regel_ider } => {
                panic!(
                    "Forventet avvis, men fikk KreverManuellVurdering: {:?}",
                    regel_ider
                )
            }
        }
    }

    fn er_godkjent(opplysninger: &[Opplysning]) -> bool {
        regelsett_v2()
            .evaluer(opplysninger)
            .is_grunnlag_for_godkjenning()
    }

    // --- Under 18 år, ikke forhåndsgodkjent ---

    #[test]
    fn under_18_avvises_selv_om_alt_annet_er_ok() {
        let res = regelsett_v2().evaluer(&[
            ErUnder18Aar,
            BosattEtterFregLoven,
            HarNorskAdresse,
            ErNorskStatsborger,
        ]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::Under18Aar]
            }
        );
    }

    #[test]
    fn under_18_avvises_med_ikke_bosatt_nar_ingen_statsborgerskap_info() {
        let res = regelsett_v2().evaluer(&[ErUnder18Aar, IkkeBosatt]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![
                    RegelId::Under18Aar,
                    RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
                ]
            }
        );
    }

    #[test]
    fn norsk_statsborger_under_18_avvises_med_ikke_bosatt() {
        let res = regelsett_v2().evaluer(&[ErUnder18Aar, IkkeBosatt, ErNorskStatsborger]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![
                    RegelId::Under18Aar,
                    RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
                ]
            }
        );
    }

    #[test]
    fn eu_eoes_under_18_avvises_med_ikke_bosatt_status() {
        let res = regelsett_v2().evaluer(&[ErUnder18Aar, IkkeBosatt, ErEuEoesStatsborger]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![
                    RegelId::Under18Aar,
                    RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt,
                ]
            }
        );
    }

    #[test]
    fn eu_eoes_under_18_med_dnummer_og_ikke_utflyttet_avvises_kun_for_alder() {
        let res = regelsett_v2().evaluer(&[
            ErUnder18Aar,
            ErEuEoesStatsborger,
            HarUtenlandskAdresse,
            HarRegistrertAdresseIEuEoes,
            IngenInformasjonOmOppholdstillatelse,
            Dnummer,
            IngenFlytteInformasjon,
        ]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::Under18Aar]
            }
        );
    }

    #[test]
    fn eu_eoes_under_18_med_dnummer_og_utflyttet_avvises_for_alder_og_ikke_bosatt_status() {
        let res = regelsett_v2().evaluer(&[
            ErUnder18Aar,
            ErEuEoesStatsborger,
            HarUtenlandskAdresse,
            HarRegistrertAdresseIEuEoes,
            IngenInformasjonOmOppholdstillatelse,
            Dnummer,
            IngenFlytteInformasjon,
            IkkeBosatt,
        ]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![
                    RegelId::Under18Aar,
                    RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt,
                ]
            }
        );
    }

    // --- Under 18 år, forhåndsgodkjent, skal avvises ---

    #[test]
    fn under_18_forhaandsgodkjent_er_doed_avvises() {
        let res = regelsett_v2().evaluer(&[
            Doed,
            ForhaandsgodkjentAvAnsatt,
            ErUnder18Aar,
            BosattEtterFregLoven,
        ]);
        assert_eq!(
            res,
            EvalueringsResultat::Avvist {
                regel_ider: vec![RegelId::Doed, RegelId::Under18Aar]
            }
        );
    }

    #[test]
    fn under_18_forhaandsgodkjent_er_savnet_avvises() {
        let res = regelsett_v2().evaluer(&[Savnet, ForhaandsgodkjentAvAnsatt, ErUnder18Aar]);
        let mut expected = vec![
            RegelId::Savnet,
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        // order-insensitive comparison: sort stringified names
        let mut got = match res {
            EvalueringsResultat::Avvist { regel_ider } => regel_ider,
            other => panic!("Forventet Avvist, men fikk {:?}", other),
        };
        got.sort_by_key(|id| format!("{:?}", id));
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(got, expected);
    }

    #[test]
    fn ikke_funnet_i_pdl_avvises() {
        let res = regelsett_v2().evaluer(&[PersonIkkeFunnet]);
        assert_eq!(
            res,
            EvalueringsResultat::Avvist {
                regel_ider: vec![RegelId::IkkeFunnet]
            }
        );
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
        let res = regelsett_v2().evaluer(&[
            ErNorskStatsborger,
            ErEuEoesStatsborger,
            ErOver18Aar,
            IkkeBosatt,
        ]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
            }
        );
    }

    #[test]
    fn over_18_tredjelandsborger_ikke_bosatt_avvises() {
        let res = regelsett_v2().evaluer(&[ErOver18Aar, IkkeBosatt]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
            }
        );
    }

    #[test]
    fn over_18_eu_eoes_ikke_norsk_med_ikke_bosatt_avvises() {
        let res = regelsett_v2().evaluer(&[ErOver18Aar, ErEuEoesStatsborger, IkkeBosatt]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt]
            }
        );
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
        let res = regelsett_v2().evaluer(&[ErOver18Aar, ErGbrStatsborger]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
            }
        );
    }

    #[test]
    fn gbr_statsborger_under_18_avvises() {
        let res = regelsett_v2().evaluer(&[ErUnder18Aar, ErGbrStatsborger]);
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        let mut got = match res {
            EvalueringsResultat::KreverManuellVurdering { regel_ider } => regel_ider,
            other => panic!("Forventet KreverManuellVurdering, men fikk {:?}", other),
        };
        got.sort_by_key(|id| format!("{:?}", id));
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(got, expected);
    }

    #[test]
    fn tredjelandsborger_under_18_avvises() {
        let res = regelsett_v2().evaluer(&[ErUnder18Aar]);
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        let mut got = match res {
            EvalueringsResultat::KreverManuellVurdering { regel_ider } => regel_ider,
            other => panic!("Forventet KreverManuellVurdering, men fikk {:?}", other),
        };
        got.sort_by_key(|id| format!("{:?}", id));
        expected.sort_by_key(|id| format!("{:?}", id));
        assert_eq!(got, expected);
    }

    #[test]
    fn tredjelandsborger_over_18_avvises_via_standardregel() {
        let res = regelsett_v2().evaluer(&[]);
        assert_eq!(
            res,
            EvalueringsResultat::KreverManuellVurdering {
                regel_ider: vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
            }
        );
    }
}
