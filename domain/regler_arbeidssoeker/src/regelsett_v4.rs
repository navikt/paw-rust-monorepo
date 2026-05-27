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
    use crate::regler::regelsett::EvalueringsResultat;
    use interne_hendelser::vo::Opplysning::*;
    use std::collections::HashSet;

    struct Evaluering<'a>(&'a [Opplysning]);

    fn gitt(opplysninger: &[Opplysning]) -> Evaluering<'_> {
        Evaluering(opplysninger)
    }

    impl Evaluering<'_> {
        fn skal_godkjennes_med(self, forventet: &[RegelId]) {
            let ids: HashSet<_> = match regelsett_v4().evaluer(self.0) {
                EvalueringsResultat::Avvist { regel_ider } => {
                    panic!(
                        "Forventet godkjenning, men fikk avvisning med regel_ider: {:?}",
                        regel_ider
                    )
                }
                EvalueringsResultat::Godkjent { regel_ider } => regel_ider.into_iter().collect(),
                EvalueringsResultat::KreverManuellVurdering { regel_ider } => {
                    panic!(
                        "Forventet GrunnlagForGodkjen, men fikk KreverManuellVurdering med regel_ider: {:?}",
                        regel_ider
                    )
                }
            };
            let forventet: HashSet<_> = forventet.iter().cloned().collect();
            assert_eq!(ids, forventet);
        }

        fn skal_avvises_med(self, forventet: &[RegelId]) {
            let ids: HashSet<_> = match regelsett_v4().evaluer(self.0) {
                EvalueringsResultat::Godkjent { regel_ider } => {
                    panic!(
                        "Forventet avvisning, men fikk GrunnlagForGodkjenning med regel_ider: {:?}",
                        regel_ider
                    )
                }
                EvalueringsResultat::Avvist { regel_ider } => regel_ider.into_iter().collect(),
                EvalueringsResultat::KreverManuellVurdering { regel_ider } => {
                    panic!(
                        "Forvent Avvist, men fikk KreverManuellVurdering med regel_ider: {:?}",
                        regel_ider
                    )
                }
            };
            let forventet: HashSet<_> = forventet.iter().cloned().collect();
            assert_eq!(ids, forventet);
        }

        fn krever_manuell_vurdering(self, forventet: &[RegelId]) {
            let ids: HashSet<_> = match regelsett_v4().evaluer(self.0) {
                EvalueringsResultat::Godkjent { regel_ider } => {
                    panic!(
                        "Forventet KreverManuellVurdering, men fikk GrunnlagForGodkjenning med regel_ider: {:?}",
                        regel_ider
                    )
                }
                EvalueringsResultat::KreverManuellVurdering { regel_ider } => {
                    regel_ider.into_iter().collect()
                }
                EvalueringsResultat::Avvist { regel_ider } => {
                    panic!(
                        "Forvent KreverManuellVurdering, men fikk Avvist med regel_ider: {:?}",
                        regel_ider
                    )
                }
            };
            let forventet: HashSet<_> = forventet.iter().cloned().collect();
            assert_eq!(ids, forventet);
        }
    }

    macro_rules! regeltest {
        () => {};
        ($navn:ident: [$($input:expr),*] => EvalueringsResultat::GrunnlagForGodkjenning([$($forventet:expr),*]), $($rest:tt)*) => {
            #[test]
            fn $navn() {
                gitt(&[$($input),*]).skal_godkjennes_med(&[$($forventet),*]);
            }
            regeltest!($($rest)*);
        };
        ($navn:ident: [$($input:expr),*] => EvalueringsResultat::KreverManuellVurdering([$($forventet:expr),*]), $($rest:tt)*) => {
            #[test]
            fn $navn() {
                gitt(&[$($input),*]).krever_manuell_vurdering(&[$($forventet),*]);
            }
            regeltest!($($rest)*);
        };
        ($navn:ident: [$($input:expr),*] => EvalueringsResultat::Avvist([$($forventet:expr),*]), $($rest:tt)*) => {
            #[test]
            fn $navn() {
                gitt(&[$($input),*]).skal_avvises_med(&[$($forventet),*]);
            }
            regeltest!($($rest)*);
        };
    }

    // --- Alltid avvist ---

    regeltest! {
        person_ikke_funnet_avvises:
            [PersonIkkeFunnet] => EvalueringsResultat::Avvist([RegelId::IkkeFunnet]),
        doed_person_avvises:
            [Doed, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar] => EvalueringsResultat::Avvist([RegelId::Doed]),
        savnet_person_avvises:
            [Savnet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar] => EvalueringsResultat::Avvist([RegelId::Savnet]),
        opphoert_identitet_avvises:
            [OpphoertIdentitet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar] => EvalueringsResultat::Avvist([RegelId::Opphoert]),
        forhaandsgodkjenning_overstyrer_ikke_doed:
            [Doed, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar] => EvalueringsResultat::Avvist([RegelId::Doed]),
        forhaandsgodkjenning_overstyrer_ikke_savnet:
            [Savnet, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar] => EvalueringsResultat::Avvist([RegelId::Savnet]),
        forhaandsgodkjenning_overstyrer_ikke_opphoert:
            [OpphoertIdentitet, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar] => EvalueringsResultat::Avvist([RegelId::Opphoert]),
    }

    // --- Under 18 år ---

    regeltest! {
        eu_eoes_under_18_bosatt_avvises_kun_for_alder:
            [ErUnder18Aar, BosattEtterFregLoven, ErEuEoesStatsborger] => EvalueringsResultat::KreverManuellVurdering([RegelId::Under18Aar]),
        under_18_tredjelandsborger_uten_bosatt_avvises_for_alder_og_bosatt:
            [ErUnder18Aar] => EvalueringsResultat::KreverManuellVurdering([RegelId::Under18Aar, RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
        under_18_eu_eoes_uten_bosatt_avvises_kun_for_alder:
            [ErUnder18Aar, ErEuEoesStatsborger] => EvalueringsResultat::KreverManuellVurdering([RegelId::Under18Aar]),
        under_18_norsk_uten_bosatt_avvises_kun_for_alder:
            [ErUnder18Aar, ErNorskStatsborger, ErEuEoesStatsborger] => EvalueringsResultat::KreverManuellVurdering([RegelId::Under18Aar]),
        under_18_forhaandsgodkjent_godkjennes:
            [ErUnder18Aar, ForhaandsgodkjentAvAnsatt] => EvalueringsResultat::GrunnlagForGodkjenning([RegelId::ForhaandsgodkjentAvAnsatt]),
        under_18_forhaandsgodkjent_doed_avvises:
            [Doed, ForhaandsgodkjentAvAnsatt, ErUnder18Aar, BosattEtterFregLoven] => EvalueringsResultat::Avvist([RegelId::Doed, RegelId::Under18Aar]),
    }

    // --- Ukjent alder ---

    regeltest! {
        ukjent_alder_tredjelandsborger_avvises_for_alder_og_bosatt:
            [UkjentFoedselsaar, UkjentFoedselsdato] => EvalueringsResultat::KreverManuellVurdering([RegelId::UkjentAlder, RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
        ukjent_alder_eu_eoes_avvises_kun_for_alder:
            [UkjentFoedselsaar, UkjentFoedselsdato, ErEuEoesStatsborger] => EvalueringsResultat::KreverManuellVurdering([RegelId::UkjentAlder]),
    }

    // --- Over 18 ---

    regeltest! {
        over_18_bosatt_godkjennes:
            [ErOver18Aar, BosattEtterFregLoven] => EvalueringsResultat::GrunnlagForGodkjenning([RegelId::Over18AarOgBosattEtterFregLoven]),
        over_18_eu_eoes_uten_bosatt_godkjennes:
            [ErOver18Aar, ErEuEoesStatsborger] => EvalueringsResultat::GrunnlagForGodkjenning([RegelId::EuEoesStatsborgerOver18Aar]),
        over_18_utflyttet_eu_eoes_godkjennes:
            [ErOver18Aar, ErEuEoesStatsborger, IkkeBosatt] => EvalueringsResultat::GrunnlagForGodkjenning([RegelId::EuEoesStatsborgerOver18Aar]),
        over_18_norsk_uten_bosatt_godkjennes:
            [ErOver18Aar, ErEuEoesStatsborger] => EvalueringsResultat::GrunnlagForGodkjenning([RegelId::EuEoesStatsborgerOver18Aar]),
        over_18_forhaandsgodkjent_godkjennes:
            [ErOver18Aar, ForhaandsgodkjentAvAnsatt] => EvalueringsResultat::GrunnlagForGodkjenning([RegelId::ForhaandsgodkjentAvAnsatt]),
        over_18_tredjelandsborger_uten_bosatt_avvises:
            [ErOver18Aar] => EvalueringsResultat::KreverManuellVurdering([RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
        over_18_gbr_statsborger_uten_bosatt_avvises:
            [ErOver18Aar, ErGbrStatsborger] => EvalueringsResultat::KreverManuellVurdering([RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
    }

    // --- Standardregel ---

    regeltest! {
        norsk_eu_eoes_uten_aldersinfo_avvises_via_standardregel:
            [ErNorskStatsborger, ErEuEoesStatsborger] => EvalueringsResultat::KreverManuellVurdering([RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
    }
}
