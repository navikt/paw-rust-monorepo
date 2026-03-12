use crate::fakta::feil::FaktaFeil;
use crate::modell::pdl::{Oppholdstillatelse, Person};
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    BarnFoedtINorgeUtenOppholdstillatelse
    , HarGyldigOppholdstillatelse, IngenInformasjonOmOppholdstillatelse,
    UkjentStatusForOppholdstillatelse,
};
use regler_core::fakta::UtledeFakta;

#[derive(Debug, Default)]
pub struct UtledeOppholdFakta;

impl UtledeFakta<Person, Opplysning> for UtledeOppholdFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        if (input.opphold.is_empty()) {
            Ok(vec![IngenInformasjonOmOppholdstillatelse])
        } else if input.opphold.len() > 1 {
            Err(FaktaFeil::FlereOppholdstillatelser(input.opphold.len()).into())
        } else {
            let opphold = &input.opphold[0];
            let fakta = match opphold.type_ {
                Oppholdstillatelse::Midlertidig => HarGyldigOppholdstillatelse,
                Oppholdstillatelse::Permanent => HarGyldigOppholdstillatelse,
                Oppholdstillatelse::OpplysningMangler => BarnFoedtINorgeUtenOppholdstillatelse,
                Oppholdstillatelse::UnknownValue => UkjentStatusForOppholdstillatelse,
            };
            Ok(vec![fakta])
        }
    }
}

#[cfg(test)]
mod tests {}
