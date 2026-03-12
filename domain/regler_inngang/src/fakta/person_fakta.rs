use crate::fakta::adresse_fakta::UtledeAdresseFakta;
use crate::fakta::alder_fakta::UtledeAlderFakta;
use crate::fakta::folkeregister_fakta::UtledeFolkeregisterFakta;
use crate::fakta::oppholdstillatelse_fakta::UtledeOppholdstillatelseFakta;
use crate::fakta::statsborgerskap_fakta::UtledeStatsborgerskapFakta;
use crate::fakta::utflytting_fakta::UtledeUtflyttingFakta;
use crate::modell::pdl::Person;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use regler_core::fakta::UtledeFakta;

#[derive(Debug)]
pub struct UtledePersonFakta {
    pub alder_fakta: UtledeAlderFakta,
    pub adresse_fakta: UtledeAdresseFakta,
    pub folkeregister_fakta: UtledeFolkeregisterFakta,
    pub statsborgerskap_fakta: UtledeStatsborgerskapFakta,
    pub opphold_fakta: UtledeOppholdstillatelseFakta,
    pub utflytting_fakta: UtledeUtflyttingFakta,
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
    use crate::modell::pdl::{
        Bostedsadresse, Foedselsdato, Folkeregisterpersonstatus, Opphold, Oppholdstillatelse,
        Person, Statsborgerskap, Vegadresse,
    };
    use chrono::NaiveDate;
    use interne_hendelser::vo::Opplysning::{
        BosattEtterFregLoven, ErEuEoesStatsborger, ErNorskStatsborger, ErOver18Aar,
        HarGyldigOppholdstillatelse, HarNorskAdresse, HarRegistrertAdresseIEuEoes,
        IngenFlytteInformasjon,
    };
    use regler_core::fakta::UtledeFakta;

    #[test]
    fn en_normal_person_gir_rette_fakta() {
        let person = Person {
            foedselsdato: vec![Foedselsdato {
                foedselsdato: NaiveDate::from_ymd_opt(1970, 1, 1),
                foedselsaar: None,
            }],
            bostedsadresse: vec![Bostedsadresse {
                vegadresse: Some(Vegadresse {
                    kommunenummer: Some("5501".to_string()),
                }),
                ..Default::default()
            }],
            statsborgerskap: vec![Statsborgerskap {
                land: "NOR".to_string(),
                ..Default::default()
            }],
            folkeregisterpersonstatus: vec![Folkeregisterpersonstatus {
                forenklet_status: "bosattEtterFolkeregisterloven".to_string(),
                ..Default::default()
            }],
            opphold: vec![Opphold {
                type_: Oppholdstillatelse::Permanent,
                ..Default::default()
            }],
            innflytting_til_norge: vec![],
            utflytting_fra_norge: vec![],
        };

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
