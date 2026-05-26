use crate::fakta::UtledeFakta;
use crate::fakta::person_fakta::UtledePersonFakta;
use crate::regelsett_v2::regelsett_v2;
use crate::regelsett_v3::regelsett_v3;
use crate::regelsett_v4::regelsett_v4;
use crate::regler::regelsett::{EvalueringsResultat, Regelsett};
use anyhow::Result;
use pdl_graphql::pdl::Person;
use serde::{Deserialize, Serialize};

struct Regelmotor {
    utlede_fakta: UtledePersonFakta,
    regelsett: Regelsett,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegelVersjon(String);

impl RegelVersjon {
    pub fn gjeldende() -> Self {
        Self(env!("REGLER_SOURCE_HASH").to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evaluering {
    pub versjon: RegelVersjon,
    pub resultat: EvalueringsResultat,
}

impl Regelmotor {
    fn v2() -> Self {
        Self {
            utlede_fakta: UtledePersonFakta::default(),
            regelsett: regelsett_v2(),
        }
    }

    fn v3() -> Self {
        Self {
            utlede_fakta: UtledePersonFakta::default(),
            regelsett: regelsett_v3(),
        }
    }

    fn v4() -> Self {
        Self {
            utlede_fakta: UtledePersonFakta::default(),
            regelsett: regelsett_v4(),
        }
    }

    pub fn evaluer(&self, person: &Person) -> Result<Evaluering> {
        let fakta = self.utlede_fakta.utlede_fakta(person)?;
        let resultat = self.regelsett.evaluer(&fakta);
        Ok(Evaluering {
            versjon: RegelVersjon::gjeldende(),
            resultat,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::regelmotor::{Evaluering, RegelVersjon, Regelmotor};
    use crate::regler::regelsett::EvalueringsResultat;
    use crate::regler::regel_id::RegelId;
    use crate::regler::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};
    use chrono::NaiveDate;
    use interne_hendelser::vo::Opplysning::{
        BosattEtterFregLoven, ErEuEoesStatsborger, ErNorskStatsborger, ErOver18Aar,
        HarGyldigOppholdstillatelse, HarNorskAdresse, HarRegistrertAdresseIEuEoes,
        IngenAdresseFunnet, IngenFlytteInformasjon, UkjentStatusForOppholdstillatelse,
    };
    use pdl_graphql::pdl::hent_person_bolk::Oppholdstillatelse;
    use pdl_graphql::pdl::{
        Bostedsadresse, Foedselsdato, Folkeregisterpersonstatus, FolkeregisterpersonstatusMetadata,
        Opphold, OppholdMetadata, Person, Statsborgerskap, StatsborgerskapMetadata, Vegadresse,
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
                foedselsdato: foedselsdato.map(|d| d.format("%Y-%m-%d").to_string()),
                foedselsaar: None,
            }],
            bostedsadresse: kommunenummer
                .iter()
                .map(|&k| Bostedsadresse {
                    angitt_flyttedato: None,
                    gyldig_fra_og_med: None,
                    gyldig_til_og_med: None,
                    vegadresse: Some(Vegadresse {
                        kommunenummer: Some(k.to_string()),
                    }),
                    matrikkeladresse: None,
                    ukjent_bosted: None,
                    utenlandsk_adresse: None,
                })
                .collect(),
            statsborgerskap: statsborgerskap
                .iter()
                .map(|&s| Statsborgerskap {
                    land: s.to_string(),
                    metadata: StatsborgerskapMetadata { endringer: vec![] },
                })
                .collect(),
            folkeregisterpersonstatus: freg_status
                .iter()
                .map(|&s| Folkeregisterpersonstatus {
                    forenklet_status: s.to_string(),
                    metadata: FolkeregisterpersonstatusMetadata { endringer: vec![] },
                })
                .collect(),
            opphold: opphold
                .iter()
                .map(|o| Opphold {
                    type_: o.clone(),
                    opphold_fra: None,
                    opphold_til: None,
                    metadata: OppholdMetadata { endringer: vec![] },
                })
                .collect(),
            innflytting_til_norge: vec![],
            utflytting_fra_norge: vec![],
        }
    }

    #[test]
    fn sjekk_regelmotor_versjon() {
        let versjon = RegelVersjon::gjeldende();
        assert_eq!(versjon.0, env!("REGLER_SOURCE_HASH").to_string());
    }

    #[test]
    fn bosatt_person_kan_godkjennes() {
        let person = person(
            NaiveDate::from_ymd_opt(1970, 1, 1),
            vec!["5501"],
            vec!["NOR"],
            vec!["bosattEtterFolkeregisterloven"],
            vec![Oppholdstillatelse::PERMANENT],
        );
        let regler_inngang: Regelmotor = Regelmotor::v2();
        let evaluering = regler_inngang.evaluer(&person).unwrap();
        assert_eq!(
            evaluering.resultat,
            EvalueringsResultat::Godkjent(vec![GrunnlagForGodkjenning {
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
            }])
        );
    }

    #[test]
    fn ikke_bosatt_person_kan_ikke_godkjennes() {
        let person = person(
            NaiveDate::from_ymd_opt(1970, 1, 1),
            vec![],
            vec!["FIJ"],
            vec![],
            vec![Oppholdstillatelse::Other("__UNKNOWN_VALUE".to_string())],
        );
        let regler_inngang: Regelmotor = Regelmotor::v2();
        let evaluering = regler_inngang.evaluer(&person).unwrap();
        assert_eq!(
            evaluering.resultat,
            EvalueringsResultat::Avvist(vec![Problem {
                regel_id: RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven,
                opplysninger: vec![
                    ErOver18Aar,
                    IngenAdresseFunnet,
                    UkjentStatusForOppholdstillatelse,
                    IngenFlytteInformasjon
                ],
                kind: ProblemKind::MuligGrunnlagForAvvisning,
            }])
        );
    }
}
