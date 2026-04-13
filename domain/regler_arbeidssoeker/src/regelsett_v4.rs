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

    fn avviste_regel_ider(opplysninger: &[Opplysning]) -> Vec<RegelId> {
        match regelsett_v4().evaluer(opplysninger) {
            Ok(_) => panic!("Forventet avvisning, men fikk godkjenning"),
            Err(problemer) => problemer.into_iter().map(|p| p.regel_id).collect(),
        }
    }

    fn er_godkjent(opplysninger: &[Opplysning]) -> bool {
        regelsett_v4().evaluer(opplysninger).is_ok()
    }

    // --- Alltid avvist ---

    #[test]
    fn person_ikke_funnet_avvises() {
        let ids = avviste_regel_ider(&[PersonIkkeFunnet]);
        assert_eq!(ids, vec![RegelId::IkkeFunnet]);
    }

    #[test]
    fn doed_person_avvises() {
        let ids = avviste_regel_ider(&[Doed, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar]);
        assert_eq!(ids, vec![RegelId::Doed]);
    }

    #[test]
    fn savnet_person_avvises() {
        let ids =
            avviste_regel_ider(&[Savnet, ErNorskStatsborger, ErEuEoesStatsborger, ErOver18Aar]);
        assert_eq!(ids, vec![RegelId::Savnet]);
    }

    #[test]
    fn opphoert_identitet_avvises() {
        let ids = avviste_regel_ider(&[
            OpphoertIdentitet,
            ErNorskStatsborger,
            ErEuEoesStatsborger,
            ErOver18Aar,
        ]);
        assert_eq!(ids, vec![RegelId::Opphoert]);
    }

    #[test]
    fn forhaandsgodkjenning_overstyrer_ikke_doed() {
        let ids = avviste_regel_ider(&[
            Doed,
            ForhaandsgodkjentAvAnsatt,
            BosattEtterFregLoven,
            ErOver18Aar,
        ]);
        assert_eq!(ids, vec![RegelId::Doed]);
    }

    #[test]
    fn forhaandsgodkjenning_overstyrer_ikke_savnet() {
        let ids = avviste_regel_ider(&[
            Savnet,
            ForhaandsgodkjentAvAnsatt,
            BosattEtterFregLoven,
            ErOver18Aar,
        ]);
        assert_eq!(ids, vec![RegelId::Savnet]);
    }

    #[test]
    fn forhaandsgodkjenning_overstyrer_ikke_opphoert() {
        let ids = avviste_regel_ider(&[
            OpphoertIdentitet,
            ForhaandsgodkjentAvAnsatt,
            BosattEtterFregLoven,
            ErOver18Aar,
        ]);
        assert_eq!(ids, vec![RegelId::Opphoert]);
    }

    // --- Under 18 år ---

    #[test]
    fn eu_eoes_under_18_bosatt_avvises_kun_for_alder() {
        let ids = avviste_regel_ider(&[
            ErUnder18Aar,
            BosattEtterFregLoven,
            ErNorskStatsborger,
            ErEuEoesStatsborger,
        ]);
        assert_eq!(ids, vec![RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_tredjelandsborger_uten_bosatt_avvises_for_alder_og_bosatt() {
        let mut ids = avviste_regel_ider(&[ErUnder18Aar]);
        ids.sort_by_key(|id| format!("{id:?}"));
        let mut expected = vec![
            RegelId::Under18Aar,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{id:?}"));
        assert_eq!(ids, expected);
    }

    #[test]
    fn under_18_eu_eoes_uten_bosatt_avvises_kun_for_alder() {
        // Tredjelandsborger-regelen gjelder ikke EU/EØS — kun aldersregelen trigger
        let ids = avviste_regel_ider(&[ErUnder18Aar, ErEuEoesStatsborger]);
        assert_eq!(ids, vec![RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_norsk_uten_bosatt_avvises_kun_for_alder() {
        // v3: norske statsborgere avvises ikke for bosettingsstatus
        let ids = avviste_regel_ider(&[ErUnder18Aar, ErNorskStatsborger, ErEuEoesStatsborger]);
        assert_eq!(ids, vec![RegelId::Under18Aar]);
    }

    #[test]
    fn under_18_forhaandsgodkjent_godkjennes() {
        assert!(er_godkjent(&[ErUnder18Aar, ForhaandsgodkjentAvAnsatt]));
    }

    #[test]
    fn under_18_forhaandsgodkjent_doed_avvises() {
        let mut ids = avviste_regel_ider(&[
            Doed,
            ForhaandsgodkjentAvAnsatt,
            ErUnder18Aar,
            BosattEtterFregLoven,
        ]);
        ids.sort_by_key(|id| format!("{id:?}"));
        let mut expected = vec![RegelId::Doed, RegelId::Under18Aar];
        expected.sort_by_key(|id| format!("{id:?}"));
        assert_eq!(ids, expected);
    }

    // --- Ukjent alder ---

    #[test]
    fn ukjent_alder_tredjelandsborger_avvises_for_alder_og_bosatt() {
        let mut ids = avviste_regel_ider(&[UkjentFoedselsaar, UkjentFoedselsdato]);
        ids.sort_by_key(|id| format!("{id:?}"));
        let mut expected = vec![
            RegelId::UkjentAlder,
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
        ];
        expected.sort_by_key(|id| format!("{id:?}"));
        assert_eq!(ids, expected);
    }

    #[test]
    fn ukjent_alder_eu_eoes_avvises_kun_for_alder() {
        let ids = avviste_regel_ider(&[UkjentFoedselsaar, UkjentFoedselsdato, ErEuEoesStatsborger]);
        assert_eq!(ids, vec![RegelId::UkjentAlder]);
    }

    // --- Over 18, godkjennes ---

    #[test]
    fn over_18_bosatt_godkjennes() {
        assert!(er_godkjent(&[ErOver18Aar, BosattEtterFregLoven]));
    }

    #[test]
    fn over_18_eu_eoes_uten_bosatt_godkjennes() {
        assert!(er_godkjent(&[ErOver18Aar, ErEuEoesStatsborger]));
    }

    #[test]
    fn over_18_norsk_uten_bosatt_godkjennes() {
        // v3: norske statsborgere behandles som andre EU/EØS — godkjennes over 18 uavhengig av bosettingsstatus
        assert!(er_godkjent(&[
            ErOver18Aar,
            ErNorskStatsborger,
            ErEuEoesStatsborger
        ]));
    }

    #[test]
    fn over_18_forhaandsgodkjent_uten_bosatt_godkjennes() {
        assert!(er_godkjent(&[ErOver18Aar, ForhaandsgodkjentAvAnsatt]));
    }

    // --- Over 18, avvises ---

    #[test]
    fn over_18_tredjelandsborger_uten_bosatt_avvises() {
        let ids = avviste_regel_ider(&[ErOver18Aar]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }

    #[test]
    fn over_18_gbr_statsborger_uten_bosatt_avvises() {
        // GBR er tredjelandsborger i v3 (ikke EU/EØS, ikke norsk)
        let ids = avviste_regel_ider(&[ErOver18Aar, ErGbrStatsborger]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }

    // --- Standardregel ---

    #[test]
    fn norsk_eu_eoes_uten_aldersinfo_avvises_via_standardregel() {
        // Ingen aldersregel treffer → standardregel aktiveres
        let ids = avviste_regel_ider(&[ErNorskStatsborger, ErEuEoesStatsborger]);
        assert_eq!(
            ids,
            vec![RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven]
        );
    }
}
