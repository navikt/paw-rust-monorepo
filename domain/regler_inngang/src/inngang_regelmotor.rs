use crate::fakta::person_fakta::UtledePersonFakta;
use crate::inngang_regelsett_v2::inngang_regelsett_v2;
use crate::inngang_regelsett_v3::inngang_regelsett_v3;
use crate::modell::feil::EvalueringFeil;
use crate::modell::pdl::Person;
use crate::regler::regelsett::Regelsett;
use crate::regler::resultat::GrunnlagForGodkjenning;
use anyhow::Result;
use regler_core::fakta::UtledeFakta;

struct InngangRegelmotor {
    utlede_fakta: UtledePersonFakta,
    regelsett: Regelsett,
}

impl InngangRegelmotor {
    fn v2() -> Self {
        Self {
            utlede_fakta: UtledePersonFakta::default(),
            regelsett: inngang_regelsett_v2(),
        }
    }

    fn v3() -> Self {
        Self {
            utlede_fakta: UtledePersonFakta::default(),
            regelsett: inngang_regelsett_v3(),
        }
    }

    pub fn evaluer(&self, person: &Person) -> Result<GrunnlagForGodkjenning> {
        let fakta = self.utlede_fakta.utlede_fakta(person)?;
        match self.regelsett.evaluer(&fakta) {
            Ok(grunnlag) => Ok(grunnlag),
            Err(problemer) => Err(EvalueringFeil::Problemer(problemer).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::inngang_regelmotor::InngangRegelmotor;
    use crate::modell::feil::EvalueringFeil;
    use crate::modell::pdl::{
        Bostedsadresse, Foedselsdato, Folkeregisterpersonstatus, Opphold, Oppholdstillatelse,
        Person, Statsborgerskap, Vegadresse,
    };
    use crate::regler::regel_id::RegelId;
    use crate::regler::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};
    use chrono::NaiveDate;
    use interne_hendelser::vo::Opplysning::{
        BosattEtterFregLoven, ErEuEoesStatsborger, ErNorskStatsborger, ErOver18Aar,
        HarGyldigOppholdstillatelse, HarNorskAdresse, HarRegistrertAdresseIEuEoes,
        IngenAdresseFunnet, IngenFlytteInformasjon, UkjentStatusForOppholdstillatelse,
    };

    fn person(
        foedselsdato: Option<NaiveDate>,
        kommunenummer: Vec<&str>,
        statsborgerskap: Vec<&str>,
        freg_status: Vec<&str>,
        opphold: Vec<Oppholdstillatelse>,
    ) -> Person {
        Person {
            foedselsdato: vec![Foedselsdato {
                foedselsdato,
                foedselsaar: None,
            }],
            bostedsadresse: kommunenummer
                .iter()
                .map(|&k| Bostedsadresse {
                    vegadresse: Some(Vegadresse {
                        kommunenummer: Some(k.to_string()),
                    }),
                    ..Default::default()
                })
                .collect(),
            statsborgerskap: statsborgerskap
                .iter()
                .map(|&s| Statsborgerskap {
                    land: s.to_string(),
                    ..Default::default()
                })
                .collect(),
            folkeregisterpersonstatus: freg_status
                .iter()
                .map(|&s| Folkeregisterpersonstatus {
                    forenklet_status: s.to_string(),
                    ..Default::default()
                })
                .collect(),
            opphold: opphold
                .iter()
                .map(|o| Opphold {
                    type_: o.clone(),
                    ..Default::default()
                })
                .collect(),
            innflytting_til_norge: vec![],
            utflytting_fra_norge: vec![],
        }
    }

    #[test]
    fn bosatt_person_kan_godkjennes() {
        let person = person(
            NaiveDate::from_ymd_opt(1970, 1, 1),
            vec!["5501"],
            vec!["NOR"],
            vec!["bosattEtterFolkeregisterloven"],
            vec![Oppholdstillatelse::Permanent],
        );
        let v2: InngangRegelmotor = InngangRegelmotor::v2();
        let result = v2.evaluer(&person);
        match result {
            Ok(grunnlag) => assert_eq!(
                grunnlag,
                GrunnlagForGodkjenning {
                    regel_id: RegelId::Over18AarOgBosattEtterFregLoven,
                    opplysninger: vec![
                        ErOver18Aar,
                        HarNorskAdresse,
                        HarRegistrertAdresseIEuEoes,
                        BosattEtterFregLoven,
                        ErNorskStatsborger,
                        ErEuEoesStatsborger,
                        HarGyldigOppholdstillatelse,
                        IngenFlytteInformasjon
                    ],
                }
            ),
            Err(error) => panic!("Forventet grunnlag for godkjenning, fikk: {:?}", error),
        }
    }

    #[test]
    fn ikke_bosatt_person_kan_ikke_godkjennes() {
        let person = person(
            NaiveDate::from_ymd_opt(1970, 1, 1),
            vec![],
            vec!["FIJ"],
            vec![],
            vec![Oppholdstillatelse::UkjentVerdi],
        );
        let v2: InngangRegelmotor = InngangRegelmotor::v2();
        let result = v2.evaluer(&person);
        match result {
            Ok(grunnlag) => panic!("Forventet problemer, fikk: {:?}", grunnlag),
            Err(error) => match error.downcast::<EvalueringFeil>() {
                Ok(EvalueringFeil::Problemer(problemer)) => {
                    assert_eq!(
                        problemer,
                        vec![Problem {
                            regel_id: RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
                            opplysninger: vec![
                                ErOver18Aar,
                                IngenAdresseFunnet,
                                UkjentStatusForOppholdstillatelse,
                                IngenFlytteInformasjon
                            ],
                            kind: ProblemKind::MuligGrunnlagForAvvisning,
                        }]
                    )
                }
                Err(other) => panic!("Forventet EvalueringFeil, fikk: {:?}", other),
            },
        }
    }
}
