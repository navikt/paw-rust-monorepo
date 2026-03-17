mod config;
pub mod person_fakta;
mod alder_fakta;
mod adresse_fakta;
mod folkeregister_fakta;
mod statsborgerskap_fakta;
mod oppholdstillatelse_fakta;
mod utflytting_fakta;

pub trait UtledeFakta<INN, UT> {
    fn utlede_fakta(&self, input: &INN) -> anyhow::Result<Vec<UT>>;
}