use crate::modell::feil::FaktaFeil;

use crate::fakta::UtledeFakta;
use crate::utils::finn_alder;
use anyhow::Result;
use chrono::NaiveDate;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    ErOver18Aar, ErUnder18Aar, UkjentFoedselsaar, UkjentFoedselsdato,
};
use pdl_graphql::pdl::Person;

#[derive(Debug, Default)]
pub struct UtledeAlderFakta;

impl UtledeFakta<Person, Opplysning> for UtledeAlderFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        if input.foedselsdato.is_empty() {
            Ok(vec![UkjentFoedselsdato, UkjentFoedselsaar])
        } else if input.foedselsdato.len() > 1 {
            Err(FaktaFeil::FlereFoedselsdatoer(input.foedselsdato.len()).into())
        } else {
            let foedselsdato = &input.foedselsdato[0];
            match foedselsdato.foedselsdato.clone() {
                Some(dato) => {
                    let naive_date = NaiveDate::parse_from_str(&dato, "%Y-%m-%d")?;
                    let alder = finn_alder(naive_date);
                    if alder > 18 {
                        Ok(vec![ErOver18Aar])
                    } else {
                        Ok(vec![ErUnder18Aar])
                    }
                }
                None => match foedselsdato.foedselsaar {
                    Some(aar) => {
                        let int = aar.to_string().parse::<i32>()?; // TODO: Finn bedre løsning
                        let foedt_dato = NaiveDate::from_ymd_opt(int, 12, 31).unwrap();
                        let alder = finn_alder(foedt_dato);
                        println!("Alder: {}", alder);
                        if alder > 18 {
                            Ok(vec![ErOver18Aar])
                        } else {
                            Ok(vec![ErUnder18Aar])
                        }
                    }
                    None => Ok(vec![UkjentFoedselsdato, UkjentFoedselsaar]),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fakta::alder_fakta::UtledeAlderFakta;
    use crate::fakta::UtledeFakta;
    use crate::modell::feil::FaktaFeil;
    use chrono::{Datelike, Local, Months};
    use interne_hendelser::vo::Opplysning::{
        ErOver18Aar, ErUnder18Aar, UkjentFoedselsaar, UkjentFoedselsdato,
    };
    use pdl_graphql::pdl::{Foedselsdato, Person};

    fn create_person(foedselsdato: Vec<Foedselsdato>) -> Person {
        Person {
            foedselsdato,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_foedselsdato_gir_ukjent_foedselsdato_og_foedselsaar() {
        let person = create_person(vec![]);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![UkjentFoedselsdato, UkjentFoedselsaar]);
    }

    #[test]
    fn mer_enn_en_foedselsdato_gir_flere_foedselsdatoer_feil() {
        let foedselsdato = vec![
            Foedselsdato {
                foedselsdato: None,
                foedselsaar: None,
            },
            Foedselsdato {
                foedselsdato: None,
                foedselsaar: None,
            },
            Foedselsdato {
                foedselsdato: None,
                foedselsaar: None,
            },
        ];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        match result {
            Ok(fakta) => panic!("Feil resultat: {:?}", fakta),
            Err(err) => assert!(matches!(
                err.downcast_ref::<FaktaFeil>(),
                Some(FaktaFeil::FlereFoedselsdatoer(3))
            )),
        };
    }

    #[test]
    fn foedselsdato_under_18_aar_gir_under_18_aar_fakta() {
        let dato = Local::now() - Months::new(12 * 17);
        let foedselsdato = vec![Foedselsdato {
            foedselsdato: Some(dato.date_naive().format("%Y-%m-%d").to_string()),
            foedselsaar: None,
        }];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![ErUnder18Aar]);
    }

    #[test]
    fn foedselsdato_over_18_aar_gir_over_18_aar_fakta() {
        let dato = Local::now() - Months::new(12 * 20);
        let foedselsdato = vec![Foedselsdato {
            foedselsdato: Some(dato.date_naive().format("%Y-%m-%d").to_string()),
            foedselsaar: None,
        }];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![ErOver18Aar]);
    }

    #[test]
    fn foedselsaar_under_18_aar_gir_under_18_aar_fakta() {
        let dato = Local::now() - Months::new(12 * 17);
        let foedselsdato = vec![Foedselsdato {
            foedselsdato: None,
            foedselsaar: Some(dato.year().into()),
        }];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![ErUnder18Aar]);
    }

    #[test]
    fn foedselsaar_over_18_aar_gir_over_18_aar_fakta() {
        let dato = Local::now() - Months::new(12 * 20);
        let foedselsdato = vec![Foedselsdato {
            foedselsdato: None,
            foedselsaar: Some(dato.year().into()),
        }];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![ErOver18Aar]);
    }
}
