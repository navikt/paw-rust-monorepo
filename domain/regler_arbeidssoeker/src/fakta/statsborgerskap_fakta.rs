use crate::fakta::config::read_regler_config;
use crate::fakta::UtledeFakta;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    ErEuEoesStatsborger, ErGbrStatsborger, ErNorskStatsborger,
};
use pdl_graphql::pdl::Person;

const NOR: &str = "NOR";
const GBR: &str = "GBR";

#[derive(Debug)]
pub struct UtledeStatsborgerskapFakta {
    eea_land: Vec<String>,
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
mod tests {
    use crate::fakta::config::read_regler_config;
    use crate::fakta::statsborgerskap_fakta::UtledeStatsborgerskapFakta;
    use crate::fakta::UtledeFakta;
    use interne_hendelser::vo::Opplysning::{
        ErEuEoesStatsborger, ErGbrStatsborger, ErNorskStatsborger,
    };
    use pdl_graphql::pdl::{Person, Statsborgerskap};

    fn create_person(land: &Vec<&str>) -> Person {
        let statsborgerskap: Vec<Statsborgerskap> = land
            .iter()
            .map(|land| Statsborgerskap {
                land: land.to_string(),
                metadata: Default::default(),
            })
            .collect();
        Person {
            statsborgerskap,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_statsborgerskap_gir_ingen_fakta() {
        let person = create_person(&vec![]);
        let result = UtledeStatsborgerskapFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert!(fakta.is_empty());
    }

    #[test]
    fn ingen_relevante_statsborgerskap_gir_ingen_fakta() {
        let land = vec!["", "123", "FJI", "RWA"];
        let person = create_person(&land);
        let result = UtledeStatsborgerskapFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert!(fakta.is_empty());
    }

    #[test]
    fn ett_norsk_statsborgerskap_gir_er_norsk_og_eea_statsborger_fakta() {
        let land = vec!["NOR"];
        let person = create_person(&land);
        let result = UtledeStatsborgerskapFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![ErNorskStatsborger, ErEuEoesStatsborger]);
    }

    #[test]
    fn ett_norsk_ett_britisk_og_andre_statsborgerskap_gir_er_norsk_britisk_og_eea_statsborger_fakta()
     {
        let land = vec!["NOR", "GBR", "FJI", "RWA"];
        let person = create_person(&land);
        let result = UtledeStatsborgerskapFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(
            fakta,
            vec![ErNorskStatsborger, ErGbrStatsborger, ErEuEoesStatsborger]
        );
    }

    #[test]
    fn ett_britisk_og_andre_statsborgerskap_gir_er_britisk_statsborger_fakta() {
        let land = vec!["GBR", "FJI", "RWA"];
        let person = create_person(&land);
        let result = UtledeStatsborgerskapFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![ErGbrStatsborger]);
    }

    #[test]
    fn ett_av_eea_statsborgerskapene_gir_er_eea_statsborger_fakta() {
        let config = read_regler_config().unwrap();
        let eea_land = config.eea_land_as_uppercase();
        for land in eea_land {
            let person = create_person(&vec![land.as_str()]);
            let result = UtledeStatsborgerskapFakta::default().utlede_fakta(&person);
            let fakta = result.unwrap();
            if land == "NOR" {
                assert_eq!(fakta, vec![ErNorskStatsborger, ErEuEoesStatsborger]);
            } else {
                assert_eq!(fakta, vec![ErEuEoesStatsborger]);
            }
        }
    }
}
