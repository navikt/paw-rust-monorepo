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
        fn skal_godkjennes_med(self, forventet: &[RegelId]) {
            let ids: HashSet<_> = match regelsett_v4().evaluer(self.0) {
                Err(_) => panic!("Forventet godkjenning, men fikk avvisning"),
                Ok(godkjenninger) => godkjenninger.into_iter().map(|g| g.regel_id).collect(),
            };
            let forventet: HashSet<_> = forventet.iter().cloned().collect();
            assert_eq!(ids, forventet);
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

    macro_rules! regeltest {
        () => {};
        ($navn:ident: [$($input:expr),*] => Ok([$($forventet:expr),*]), $($rest:tt)*) => {
            #[test]
            fn $navn() {
                gitt(&[$($input),*]).skal_godkjennes_med(&[$($forventet),*]);
            }
            regeltest!($($rest)*);
        };
        ($navn:ident: [$($input:expr),*] => Err([$($forventet:expr),*]), $($rest:tt)*) => {
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
            [PersonIkkeFunnet] => Err([RegelId::IkkeFunnet]),
        doed_person_avvises:
            [Doed, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar] => Err([RegelId::Doed]),
        savnet_person_avvises:
            [Savnet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar] => Err([RegelId::Savnet]),
        opphoert_identitet_avvises:
            [OpphoertIdentitet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar] => Err([RegelId::Opphoert]),
        forhaandsgodkjenning_overstyrer_ikke_doed:
            [Doed, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar] => Err([RegelId::Doed]),
        forhaandsgodkjenning_overstyrer_ikke_savnet:
            [Savnet, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar] => Err([RegelId::Savnet]),
        forhaandsgodkjenning_overstyrer_ikke_opphoert:
            [OpphoertIdentitet, ForhaandsgodkjentAvAnsatt, BosattEtterFregLoven, ErOver18Aar] => Err([RegelId::Opphoert]),
    }

    // --- Under 18 år ---

    regeltest! {
        eu_eoes_under_18_bosatt_avvises_kun_for_alder:
            [ErUnder18Aar, BosattEtterFregLoven, ErEuEoesStatsborger] => Err([RegelId::Under18Aar]),
        under_18_tredjelandsborger_uten_bosatt_avvises_for_alder_og_bosatt:
            [ErUnder18Aar] => Err([RegelId::Under18Aar, RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
        under_18_eu_eoes_uten_bosatt_avvises_kun_for_alder:
            [ErUnder18Aar, ErEuEoesStatsborger] => Err([RegelId::Under18Aar]),
        under_18_norsk_uten_bosatt_avvises_kun_for_alder:
            [ErUnder18Aar, ErNorskStatsborger, ErEuEoesStatsborger] => Err([RegelId::Under18Aar]),
        under_18_forhaandsgodkjent_godkjennes:
            [ErUnder18Aar, ForhaandsgodkjentAvAnsatt] => Ok([RegelId::ForhaandsgodkjentAvAnsatt]),
        under_18_forhaandsgodkjent_doed_avvises:
            [Doed, ForhaandsgodkjentAvAnsatt, ErUnder18Aar, BosattEtterFregLoven] => Err([RegelId::Doed, RegelId::Under18Aar]),
    }

    // --- Ukjent alder ---

    regeltest! {
        ukjent_alder_tredjelandsborger_avvises_for_alder_og_bosatt:
            [UkjentFoedselsaar, UkjentFoedselsdato] => Err([RegelId::UkjentAlder, RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
        ukjent_alder_eu_eoes_avvises_kun_for_alder:
            [UkjentFoedselsaar, UkjentFoedselsdato, ErEuEoesStatsborger] => Err([RegelId::UkjentAlder]),
    }

    // --- Over 18 ---

    regeltest! {
        over_18_bosatt_godkjennes:
            [ErOver18Aar, BosattEtterFregLoven] => Ok([RegelId::Over18AarOgBosattEtterFregLoven]),
        over_18_eu_eoes_uten_bosatt_godkjennes:
            [ErOver18Aar, ErEuEoesStatsborger] => Ok([RegelId::EuEoesStatsborgerOver18Aar]),
        over_18_utflyttet_eu_eoes_godkjennes:
            [ErOver18Aar, ErEuEoesStatsborger, IkkeBosatt] => Ok([RegelId::EuEoesStatsborgerOver18Aar]),
        over_18_norsk_uten_bosatt_godkjennes:
            [ErOver18Aar, ErEuEoesStatsborger] => Ok([RegelId::EuEoesStatsborgerOver18Aar]),
        over_18_forhaandsgodkjent_godkjennes:
            [ErOver18Aar, ForhaandsgodkjentAvAnsatt] => Ok([RegelId::ForhaandsgodkjentAvAnsatt]),
        over_18_tredjelandsborger_uten_bosatt_avvises:
            [ErOver18Aar] => Err([RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
        over_18_gbr_statsborger_uten_bosatt_avvises:
            [ErOver18Aar, ErGbrStatsborger] => Err([RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
    }

    // --- Standardregel ---

    regeltest! {
        norsk_eu_eoes_uten_aldersinfo_avvises_via_standardregel:
            [ErNorskStatsborger, ErEuEoesStatsborger] => Err([RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]),
    }
}
