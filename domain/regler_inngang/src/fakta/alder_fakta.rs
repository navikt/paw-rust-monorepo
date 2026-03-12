use crate::fakta::feil::FaktaFeil;

use crate::modell::pdl::Person;
use crate::utils::finn_alder;
use anyhow::Result;
use chrono::NaiveDate;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    ErOver18Aar, ErUnder18Aar, UkjentFoedselsaar, UkjentFoedselsdato,
};
use regler_core::fakta::UtledeFakta;

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
        }
    }
}

#[cfg(test)]
mod tests {}
