use crate::fakta::adresse_fakta::UtledeAdresseFakta;
use crate::fakta::alder_fakta::UtledeAlderFakta;
use crate::fakta::folkeregister_fakta::UtledeFolkeregisterFakta;
use crate::fakta::opphold_fakta::UtledeOppholdFakta;
use crate::fakta::statsborgerskap_fakta::UtledeStatsborgerskapFakta;
use crate::fakta::utflytting_fakta::UtledeUtflyttingFakta;
use crate::modell::pdl::Person;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use regler_core::fakta::UtledeFakta;

#[derive(Debug)]
pub struct UtledePersonFakta {
    pub alder_fakta: UtledeAlderFakta,
    pub adresse_fakta: UtledeAdresseFakta,
    pub folkeregister_fakta: UtledeFolkeregisterFakta,
    pub statsborgerskap_fakta: UtledeStatsborgerskapFakta,
    pub opphold_fakta: UtledeOppholdFakta,
    pub utflytting_fakta: UtledeUtflyttingFakta,
}

impl Default for UtledePersonFakta {
    fn default() -> Self {
        Self {
            alder_fakta: UtledeAlderFakta::default(),
            adresse_fakta: UtledeAdresseFakta::default(),
            folkeregister_fakta: UtledeFolkeregisterFakta::default(),
            statsborgerskap_fakta: UtledeStatsborgerskapFakta::default(),
            opphold_fakta: UtledeOppholdFakta::default(),
            utflytting_fakta: UtledeUtflyttingFakta::default(),
        }
    }
}

impl UtledeFakta<Person, Opplysning> for UtledePersonFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let mut fakta = vec![];
        fakta.append(&mut self.alder_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.adresse_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.folkeregister_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.statsborgerskap_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.opphold_fakta.utlede_fakta(&input)?);
        fakta.append(&mut self.utflytting_fakta.utlede_fakta(&input)?);
        Ok(fakta)
    }
}

#[cfg(test)]
mod tests {}
