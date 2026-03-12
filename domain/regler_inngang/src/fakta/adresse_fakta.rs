use crate::fakta::feil::FaktaFeil;

use crate::fakta::config::read_regler_config;
use crate::modell::pdl::Person;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    HarNorskAdresse, HarRegistrertAdresseIEuEoes, HarUtenlandskAdresse, IngenAdresseFunnet,
};
use regler_core::fakta::UtledeFakta;

#[derive(Debug)]
pub struct UtledeAdresseFakta {
    pub eea_land: Vec<String>,
}

impl Default for UtledeAdresseFakta {
    fn default() -> Self {
        let config = read_regler_config().unwrap();
        Self {
            eea_land: config.eea_land_as_uppercase(),
        }
    }
}

impl UtledeFakta<Person, Opplysning> for UtledeAdresseFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        if input.bostedsadresse.is_empty() {
            Ok(vec![IngenAdresseFunnet])
        } else if input.bostedsadresse.len() > 1 {
            Err(FaktaFeil::FlereBostedsadresser(input.bostedsadresse.len()).into())
        } else {
            let bostedsadresse = &input.bostedsadresse[0];
            let vegadresse = bostedsadresse.vegadresse.as_ref();
            let matrikkeladresse = bostedsadresse.matrikkeladresse.as_ref();
            let ukjent_bosted = bostedsadresse.ukjent_bosted.as_ref();
            let utenlandsk_adresse = bostedsadresse.utenlandsk_adresse.as_ref();
            if vegadresse.is_some_and(|adresse| adresse.kommunenummer.is_some()) {
                return Ok(vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
            }
            if matrikkeladresse.is_some_and(|adresse| adresse.kommunenummer.is_some()) {
                return Ok(vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
            }
            if ukjent_bosted.is_some_and(|bosted| bosted.bostedskommune.is_some()) {
                return Ok(vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
            }
            if let Some(adresse) = utenlandsk_adresse {
                return if self.eea_land.contains(&adresse.landkode.to_uppercase()) {
                    Ok(vec![HarUtenlandskAdresse, HarRegistrertAdresseIEuEoes])
                } else {
                    Ok(vec![HarUtenlandskAdresse])
                };
            }

            Ok(vec![IngenAdresseFunnet])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modell::pdl::{
        Bostedsadresse, Matrikkeladresse, UkjentBosted, UtenlandskAdresse, Vegadresse,
    };

    fn create_person(adresser: Vec<Bostedsadresse>) -> Person {
        Person {
            bostedsadresse: adresser,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_adresse_gir_ingen_adresse_funnet() {
        let person = create_person(vec![]);
        match UtledeAdresseFakta::default().utlede_fakta(&person) {
            Ok(fakta) => assert_eq!(fakta, vec![IngenAdresseFunnet]),
            Err(err) => panic!("Feil resultat: {}", err),
        }
    }

    #[test]
    fn mer_enn_en_adresse_gir_ingen_adresse_funnet() {
        let adresser = vec![Bostedsadresse {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: None,
            matrikkeladresse: None,
            ukjent_bosted: None,
            utenlandsk_adresse: None,
        }];
        let person = create_person(adresser);
        match UtledeAdresseFakta::default().utlede_fakta(&person) {
            Ok(fakta) => panic!("Feil resultat: {:?}", fakta),
            Err(err) => assert!(matches!(
                err.downcast_ref::<FaktaFeil>(),
                Some(FaktaFeil::FlereBostedsadresser(2))
            )),
        };
    }

    #[test]
    fn har_vegadresse_gir_norsk_adresse() {
        let adresser = vec![Bostedsadresse {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: Some(Vegadresse {
                kommunenummer: Some("1201".to_string()),
            }),
            matrikkeladresse: None,
            ukjent_bosted: None,
            utenlandsk_adresse: None,
        }];
        let person = create_person(adresser);
        let fakta = UtledeAdresseFakta::default().utlede_fakta(&person).unwrap();
        assert_eq!(fakta, vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
    }

    #[test]
    fn har_matrikkeladresse_gir_norsk_adresse() {
        let adresser = vec![Bostedsadresse {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: None,
            matrikkeladresse: Some(Matrikkeladresse {
                kommunenummer: Some("1201".to_string()),
            }),
            ukjent_bosted: None,
            utenlandsk_adresse: None,
        }];
        let person = create_person(adresser);
        let fakta = UtledeAdresseFakta::default().utlede_fakta(&person).unwrap();
        assert_eq!(fakta, vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
    }

    #[test]
    fn har_ukjent_bosted_gir_norsk_adresse() {
        let adresser = vec![Bostedsadresse {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: None,
            matrikkeladresse: None,
            ukjent_bosted: Some(UkjentBosted {
                bostedskommune: Some("1201".to_string()),
            }),
            utenlandsk_adresse: None,
        }];
        let person = create_person(adresser);
        let fakta = UtledeAdresseFakta::default().utlede_fakta(&person).unwrap();
        assert_eq!(fakta, vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
    }

    #[test]
    fn har_utenlandsk_adresse_i_eu_eoes_gir_registrert_adresse_i_eu_eoes() {
        let adresser = vec![Bostedsadresse {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: None,
            matrikkeladresse: None,
            ukjent_bosted: None,
            utenlandsk_adresse: Some(UtenlandskAdresse {
                landkode: "SWE".to_string(),
            }),
        }];
        let person = create_person(adresser);
        let fakta = UtledeAdresseFakta::default().utlede_fakta(&person).unwrap();
        assert_eq!(
            fakta,
            vec![HarUtenlandskAdresse, HarRegistrertAdresseIEuEoes]
        );
    }

    #[test]
    fn har_utenlandsk_adresse_utenfor_eu_eoes_gir_kun_utenlandsk_adresse() {
        let adresser = vec![Bostedsadresse {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: None,
            matrikkeladresse: None,
            ukjent_bosted: None,
            utenlandsk_adresse: Some(UtenlandskAdresse {
                landkode: "USA".to_string(),
            }),
        }];
        let person = create_person(adresser);
        let fakta = UtledeAdresseFakta::default().utlede_fakta(&person).unwrap();
        assert_eq!(fakta, vec![HarUtenlandskAdresse]);
    }
}
