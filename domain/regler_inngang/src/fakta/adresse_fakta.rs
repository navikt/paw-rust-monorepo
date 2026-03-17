use crate::modell::feil::FaktaFeil;

use crate::fakta::config::read_regler_config;
use crate::fakta::UtledeFakta;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    HarNorskAdresse, HarRegistrertAdresseIEuEoes, HarUtenlandskAdresse, IngenAdresseFunnet,
};
use pdl_graphql::pdl::Person;

#[derive(Debug)]
pub struct UtledeAdresseFakta {
    eea_land: Vec<String>,
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
    use pdl_graphql::pdl::{
        Bostedsadresse, Matrikkeladresse, UkjentBosted, UtenlandskAdresse, Vegadresse,
    };

    fn create_person(bostedsadresse: Vec<Bostedsadresse>) -> Person {
        Person {
            bostedsadresse,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_adresse_gir_ingen_adresse_funnet_fakta() {
        let person = create_person(vec![]);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![IngenAdresseFunnet]);
    }

    #[test]
    fn mer_enn_en_adresse_gir_flere_adresser_feil() {
        let adresser = vec![Bostedsadresse::default(), Bostedsadresse::default()];
        let person = create_person(adresser);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        match result {
            Ok(fakta) => panic!("Feil resultat: {:?}", fakta),
            Err(err) => assert!(matches!(
                err.downcast_ref::<FaktaFeil>(),
                Some(FaktaFeil::FlereBostedsadresser(2))
            )),
        };
    }

    #[test]
    fn har_vegadresse_gir_norsk_og_eea_adresse_fakta() {
        let adresser = vec![Bostedsadresse {
            vegadresse: Some(Vegadresse {
                kommunenummer: Some("5501".to_string()),
            }),
            ..Default::default()
        }];
        let person = create_person(adresser);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
    }

    #[test]
    fn har_matrikkeladresse_gir_norsk_og_eea_adresse_fakta() {
        let adresser = vec![Bostedsadresse {
            matrikkeladresse: Some(Matrikkeladresse {
                kommunenummer: Some("5501".to_string()),
            }),
            ..Default::default()
        }];
        let person = create_person(adresser);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
    }

    #[test]
    fn har_ukjent_bosted_gir_norsk_og_eea_adresse_fakta() {
        let adresser = vec![Bostedsadresse {
            ukjent_bosted: Some(UkjentBosted {
                bostedskommune: Some("5501".to_string()),
            }),
            ..Default::default()
        }];
        let person = create_person(adresser);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![HarNorskAdresse, HarRegistrertAdresseIEuEoes]);
    }

    #[test]
    fn har_utenlandsk_adresse_i_eu_eoes_gir_utenlandsk_og_eea_adresse_fakta() {
        let adresser = vec![Bostedsadresse {
            utenlandsk_adresse: Some(UtenlandskAdresse {
                landkode: "ITA".to_string(),
            }),
            ..Default::default()
        }];
        let person = create_person(adresser);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(
            fakta,
            vec![HarUtenlandskAdresse, HarRegistrertAdresseIEuEoes]
        );
    }

    #[test]
    fn har_utenlandsk_adresse_utenfor_eu_eoes_gir_kun_utenlandsk_adresse_fakta() {
        let adresser = vec![Bostedsadresse {
            utenlandsk_adresse: Some(UtenlandskAdresse {
                landkode: "RWA".to_string(),
            }),
            ..Default::default()
        }];
        let person = create_person(adresser);
        let result = UtledeAdresseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![HarUtenlandskAdresse]);
    }
}
