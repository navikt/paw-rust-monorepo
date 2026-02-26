use crate::fakta::feil::FaktaFeil;
use crate::modell::pdl::Person;
use crate::utils::finn_alder;
use anyhow::Result;
use chrono::NaiveDate;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    ErOver18Aar, ErUnder18Aar, HarNorskAdresse, HarRegistrertAdresseIEuEoes, HarUtenlandskAdresse,
    IngenAdresseFunnet, UkjentFoedselsaar, UkjentFoedselsdato,
};
use regler_core::fakta::UtledeFakta;

pub struct UtledePersonFakta {}

impl UtledePersonFakta {
    fn utlede_alder_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        if input.foedselsdato.len() == 0 {
            Ok(vec![UkjentFoedselsdato, UkjentFoedselsaar])
        } else if input.foedselsdato.len() == 1 {
            let foedselsdato = &input.foedselsdato[0];
            match foedselsdato.foedselsdato {
                Some(dato) => {
                    let alder = finn_alder(dato);
                    if alder > 18 {
                        Ok(vec![ErOver18Aar])
                    } else {
                        Ok(vec![ErUnder18Aar])
                    }
                }
                None => match foedselsdato.foedselsaar {
                    Some(aar) => {
                        let foedt_dato = NaiveDate::from_ymd_opt(aar, 12, 31).unwrap();
                        let alder = finn_alder(foedt_dato);
                        if alder > 18 {
                            Ok(vec![ErOver18Aar])
                        } else {
                            Ok(vec![ErUnder18Aar])
                        }
                    }
                    None => Ok(vec![UkjentFoedselsdato, UkjentFoedselsaar]),
                },
            }
        } else {
            Err(FaktaFeil::FlereFoedselsdatoer(input.foedselsdato.len()).into())
        }
    }

    fn utlede_adresse_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        if input.bostedsadresse.len() == 0 {
            Ok(vec![IngenAdresseFunnet])
        } else if input.bostedsadresse.len() == 1 {
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
            if utenlandsk_adresse.is_some() {
                let adresse = utenlandsk_adresse.unwrap();
                return if adresse.landkode == "NOR" {
                    Ok(vec![HarUtenlandskAdresse, HarRegistrertAdresseIEuEoes])
                } else {
                    Ok(vec![HarUtenlandskAdresse])
                };
            }

            Ok(vec![IngenAdresseFunnet])
        } else {
            Err(FaktaFeil::FlereBostedsadresse(input.bostedsadresse.len()).into())
        }
    }
}

impl UtledeFakta<Person, Opplysning> for UtledePersonFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let mut fakta = vec![];
        fakta.append(&mut self.utlede_alder_fakta(input)?);
        fakta.append(&mut self.utlede_adresse_fakta(input)?);
        Ok(fakta)
    }
}
