use crate::modell::pdl::Person;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    BosattEtterFregLoven, Dnummer, Doed, IkkeBosatt, OpphoertIdentitet, Savnet,
    UkjentForenkletFregStatus,
};
use regler_core::fakta::UtledeFakta;
use std::collections::HashMap;

const BOSATT: &str = "bosattEtterFolkeregisterloven";
const IKKE_BOSATT: &str = "ikkeBosatt";
const DOED: &str = "doedIFolkeregisteret";
const FORSVUNNET: &str = "forsvunnet";
const OPPHOERT: &str = "opphoert";
const D_NUMMER: &str = "dNummer";

#[derive(Debug)]
pub struct UtledeFolkeregisterFakta {
    pub status_map: HashMap<String, Opplysning>,
}

impl Default for UtledeFolkeregisterFakta {
    fn default() -> Self {
        Self {
            status_map: HashMap::from([
                (BOSATT.to_uppercase(), BosattEtterFregLoven),
                (IKKE_BOSATT.to_uppercase(), IkkeBosatt),
                (DOED.to_uppercase(), Doed),
                (FORSVUNNET.to_uppercase(), Savnet),
                (OPPHOERT.to_uppercase(), OpphoertIdentitet),
                (D_NUMMER.to_uppercase(), Dnummer),
            ]),
        }
    }
}

impl UtledeFakta<Person, Opplysning> for UtledeFolkeregisterFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let mut fakta = vec![];
        for status in &input.folkeregisterpersonstatus {
            let key = status.forenklet_status.to_uppercase();
            fakta.push(
                self.status_map
                    .get(key.as_str())
                    .unwrap_or(&UkjentForenkletFregStatus)
                    .clone(),
            );
        }
        Ok(fakta)
    }
}

#[cfg(test)]
mod tests {}
