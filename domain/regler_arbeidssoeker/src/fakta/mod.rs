mod adresse_fakta;
mod alder_fakta;
mod config;
mod folkeregister_fakta;
mod oppholdstillatelse_fakta;
pub mod person_fakta;
mod statsborgerskap_fakta;
mod utflytting_fakta;

use crate::modell::feil::FaktaFeil;

pub trait UtledeFakta<INN, UT> {
    fn utlede_fakta(&self, input: &INN) -> Result<Vec<UT>, FaktaFeil>;

    fn utlede_fakta_liste<'a, K>(
        &self,
        input: &'a [(K, INN)],
    ) -> Vec<(&'a K, Result<Vec<UT>, FaktaFeil>)> {
        input
            .iter()
            .map(|(key, value)| (key, self.utlede_fakta(value)))
            .collect()
    }
}

