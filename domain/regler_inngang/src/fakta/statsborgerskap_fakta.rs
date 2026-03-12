use crate::fakta::config::read_regler_config;
use crate::modell::pdl::Person;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    ErEuEoesStatsborger, ErGbrStatsborger, ErNorskStatsborger,
};
use regler_core::fakta::UtledeFakta;

const NOR: &str = "NOR";
const GBR: &str = "GBR";

#[derive(Debug)]
pub struct UtledeStatsborgerskapFakta {
    pub eea_land: Vec<String>,
}

impl Default for UtledeStatsborgerskapFakta {
    fn default() -> Self {
        let config = read_regler_config().unwrap();
        Self {
            eea_land: config.eea_land_as_uppercase(),
        }
    }
}

impl UtledeFakta<Person, Opplysning> for UtledeStatsborgerskapFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let mut fakta = vec![];
        for statsborgerskap in &input.statsborgerskap {
            if statsborgerskap.land.to_uppercase() == NOR {
                fakta.push(ErNorskStatsborger);
                break;
            }
        }
        for statsborgerskap in &input.statsborgerskap {
            if statsborgerskap.land.to_uppercase() == GBR {
                fakta.push(ErGbrStatsborger);
                break;
            }
        }
        for statsborgerskap in &input.statsborgerskap {
            if self.eea_land.contains(&statsborgerskap.land.to_uppercase()) {
                fakta.push(ErEuEoesStatsborger);
                break;
            }
        }
        Ok(fakta)
    }
}

#[cfg(test)]
mod tests {}
