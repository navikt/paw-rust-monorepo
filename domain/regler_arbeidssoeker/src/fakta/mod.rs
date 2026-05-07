mod adresse_fakta;
mod alder_fakta;
mod config;
mod folkeregister_fakta;
mod oppholdstillatelse_fakta;
pub mod person_fakta;
mod statsborgerskap_fakta;
mod utflytting_fakta;

pub trait UtledeFakta<INN, UT> {
    fn utlede_fakta(&self, input: &INN) -> anyhow::Result<Vec<UT>>;

    fn utlede_fakta_liste<'a, K>(
        &self,
        input: &'a [(K, INN)],
    ) -> Vec<(&'a K, anyhow::Result<Vec<UT>>)> {
        input
            .iter()
            .map(|(key, value)| (key, self.utlede_fakta(value)))
            .collect()
    }
}

