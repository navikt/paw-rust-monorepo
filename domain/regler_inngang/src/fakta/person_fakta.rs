use crate::fakta::adresse_fakta::UtledeAdresseFakta;
use crate::fakta::alder_fakta::UtledeAlderFakta;
use crate::fakta::folkeregister_fakta::UtledeFolkeregisterFakta;
use crate::fakta::oppholdstillatelse_fakta::UtledeOppholdstillatelseFakta;
use crate::fakta::statsborgerskap_fakta::UtledeStatsborgerskapFakta;
use crate::fakta::utflytting_fakta::UtledeUtflyttingFakta;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use pdl_graphql::pdl::Person;
use regler_core::fakta::UtledeFakta;

#[derive(Debug)]
pub struct UtledePersonFakta {
    alder_fakta: UtledeAlderFakta,
    adresse_fakta: UtledeAdresseFakta,
    folkeregister_fakta: UtledeFolkeregisterFakta,
    statsborgerskap_fakta: UtledeStatsborgerskapFakta,
    opphold_fakta: UtledeOppholdstillatelseFakta,
    utflytting_fakta: UtledeUtflyttingFakta,
}

impl Default for UtledePersonFakta {
    fn default() -> Self {
        Self {
            alder_fakta: UtledeAlderFakta::default(),
            adresse_fakta: UtledeAdresseFakta::default(),
            folkeregister_fakta: UtledeFolkeregisterFakta::default(),
            statsborgerskap_fakta: UtledeStatsborgerskapFakta::default(),
            opphold_fakta: UtledeOppholdstillatelseFakta::default(),
            utflytting_fakta: UtledeUtflyttingFakta::default(),
        }
    }
}

impl UtledeFakta<Person, Opplysning> for UtledePersonFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let mut fakta = vec![];
        fakta.append(&mut self.alder_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.adresse_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.folkeregister_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.statsborgerskap_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.opphold_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.utflytting_fakta.utlede_fakta(&input)?);
        Ok(fakta)
    }
}

#[cfg(test)]
mod tests {
    use crate::fakta::person_fakta::UtledePersonFakta;
    use chrono::NaiveDate;
    use interne_hendelser::vo::Opplysning::{
        BosattEtterFregLoven, ErEuEoesStatsborger, ErNorskStatsborger, ErOver18Aar,
        HarGyldigOppholdstillatelse, HarNorskAdresse, HarRegistrertAdresseIEuEoes,
        IngenFlytteInformasjon,
    };
    use pdl_graphql::pdl::hent_person_bolk::Oppholdstillatelse;
    use pdl_graphql::pdl::{
        Bostedsadresse, Foedselsdato, Folkeregisterpersonstatus, Opphold, Person, Statsborgerskap,
        Vegadresse,
    };
    use regler_core::fakta::UtledeFakta;

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
    fn en_normal_person_gir_rette_fakta() {
        let person = person(
            NaiveDate::from_ymd_opt(1970, 1, 1),
            vec!["5501"],
            vec!["NOR"],
            vec!["bosattEtterFolkeregisterloven"],
            vec![Oppholdstillatelse::PERMANENT],
        );

        let result = UtledePersonFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(
            fakta,
            vec![
                ErOver18Aar,
                HarNorskAdresse,
                HarRegistrertAdresseIEuEoes,
                BosattEtterFregLoven,
                ErNorskStatsborger,
                ErEuEoesStatsborger,
                HarGyldigOppholdstillatelse,
                IngenFlytteInformasjon
            ]
        );
    }
}
