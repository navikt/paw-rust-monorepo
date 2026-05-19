use std::collections::HashSet;
use std::vec;

use crate::modell::feil::FaktaFeil;

use crate::fakta::UtledeFakta;
use crate::utils::finn_alder;
use anyhow::{Context, Error, Result};
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
        let res: Result<Vec<Vec<Opplysning>>, Error> = input
            .foedselsdato
            .iter()
            .map(|foedsels_info| {
                let info = (
                    foedsels_info.foedselsdato.clone(),
                    foedsels_info.foedselsaar.clone(),
                );
                match info {
                    (Some(dato), _) => {
                        let naive_date = NaiveDate::parse_from_str(&dato, "%Y-%m-%d")
                            .context("Invalid date format")?;
                        let alder = finn_alder(naive_date);
                        if alder > 18 {
                            Ok(vec![ErOver18Aar])
                        } else {
                            Ok(vec![ErUnder18Aar])
                        }
                    }
                    (None, Some(aar)) => {
                        let aar = i32::try_from(aar).context("Invalid year, out of i32 range")?;
                        let foedt_dato = NaiveDate::from_ymd_opt(aar, 12, 31).unwrap();
                        let alder = finn_alder(foedt_dato);
                        println!("Alder: {}", alder);
                        if alder > 18 {
                            Ok(vec![ErOver18Aar, UkjentFoedselsdato])
                        } else {
                            Ok(vec![ErUnder18Aar, UkjentFoedselsdato])
                        }
                    }
                    (None, None) => Ok(vec![UkjentFoedselsdato, UkjentFoedselsaar]),
                }
            })
            .collect();
        let res = res?;
        if res.is_empty() {
            return Ok(vec![UkjentFoedselsaar, UkjentFoedselsdato]);
        }
        if res.iter().all(|r| r.contains(&ErOver18Aar)) {
            return Ok(vec![ErOver18Aar]);
        }
        if res
            .iter()
            .any(|r| r.contains(&UkjentFoedselsdato) && r.contains(&UkjentFoedselsaar))
        {
            return Ok(vec![UkjentFoedselsdato, UkjentFoedselsaar]);
        }
        if res.iter().any(|r| r.contains(&ErUnder18Aar)) {
            return Ok(vec![ErUnder18Aar]);
        }
        return Ok(vec![UkjentFoedselsaar, UkjentFoedselsdato]);
    }
}

#[cfg(test)]
mod tests {
    use crate::fakta::UtledeFakta;
    use crate::fakta::alder_fakta::UtledeAlderFakta;
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

    fn date_to_string(date: chrono::DateTime<Local>) -> String {
        date.format("%Y-%m-%d").to_string()
    }

    #[test]
    fn ingen_foedselsdato_gir_ukjent_foedselsdato_og_foedselsaar() {
        let person = create_person(vec![]);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![UkjentFoedselsaar, UkjentFoedselsdato]);
    }

    #[test]
    fn en_alder_under_18_og_to_over_skal_gi_under_18() {
        let dato_under_1 = Local::now() - Months::new(12 * 17);
        let dato_under_2 = Local::now() - Months::new(12 * 16);
        let dato_over_1 = Local::now() - Months::new(12 * 18);
        let foedselsdato = vec![
            Foedselsdato {
                foedselsdato: Some(date_to_string(dato_under_1)),
                foedselsaar: Some(dato_under_1.year().into()),
            },
            Foedselsdato {
                foedselsdato: Some(date_to_string(dato_over_1)),
                foedselsaar: None,
            },
            Foedselsdato {
                foedselsdato: Some(date_to_string(dato_under_2)),
                foedselsaar: None,
            },
        ];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        match result {
            Err(err) => panic!("Feil resultat: {:?}", err),
            Ok(opplysninger) => {
                assert_eq!(opplysninger, vec![ErUnder18Aar])
            }
        };
    }

    #[test]
    fn en_alder_over_18_en_under_og_en_med_none_skal_gi_alder_mangler() {
        let dato_under = Local::now() - Months::new(12 * 16);
        let dato_over = Local::now() - Months::new(12 * 18);
        let foedselsdato = vec![
            Foedselsdato {
                foedselsdato: None,
                foedselsaar: None,
            },
            Foedselsdato {
                foedselsdato: Some(date_to_string(dato_over)),
                foedselsaar: None,
            },
            Foedselsdato {
                foedselsdato: Some(date_to_string(dato_under)),
                foedselsaar: None,
            },
        ];
        let person = create_person(foedselsdato);
        let result = UtledeAlderFakta::default().utlede_fakta(&person);
        match result {
            Err(err) => panic!("Feil resultat: {:?}", err),
            Ok(opplysninger) => {
                assert_eq!(opplysninger, vec![UkjentFoedselsdato, UkjentFoedselsaar])
            }
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
