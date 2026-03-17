use crate::fakta::UtledeFakta;
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    BosattEtterFregLoven, Dnummer, Doed, IkkeBosatt, OpphoertIdentitet, Savnet,
    UkjentForenkletFregStatus,
};
use pdl_graphql::pdl::Person;
use std::collections::HashMap;

const BOSATT: &str = "bosattEtterFolkeregisterloven";
const IKKE_BOSATT: &str = "ikkeBosatt";
const DOED: &str = "doedIFolkeregisteret";
const FORSVUNNET: &str = "forsvunnet";
const OPPHOERT: &str = "opphoert";
const D_NUMMER: &str = "dNummer";

#[derive(Debug)]
pub struct UtledeFolkeregisterFakta {
    status_map: HashMap<String, Opplysning>,
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
        fakta.dedup(); // Fjerne duplikater
        Ok(fakta)
    }
}

#[cfg(test)]
mod tests {
    use crate::fakta::folkeregister_fakta::UtledeFolkeregisterFakta;
    use crate::fakta::UtledeFakta;
    use interne_hendelser::vo::Opplysning::{
        BosattEtterFregLoven, Dnummer, Doed, IkkeBosatt, OpphoertIdentitet, Savnet,
    };
    use pdl_graphql::pdl::{Folkeregisterpersonstatus, Person};

    fn create_person(status: Vec<&str>) -> Person {
        let folkeregisterpersonstatus = status
            .into_iter()
            .map(|s| Folkeregisterpersonstatus {
                forenklet_status: s.to_string(),
                ..Default::default()
            })
            .collect();
        Person {
            folkeregisterpersonstatus,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_freg_statuser_gir_ingen_fakta() {
        let person = create_person(vec![]);
        let result = UtledeFolkeregisterFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert!(fakta.is_empty());
    }

    #[test]
    fn ett_par_freg_statuser_gir_freg_fakta() {
        let person = create_person(vec![
            "bosattEtterFolkeregisterloven",
            "doedIFolkeregisteret",
        ]);
        let result = UtledeFolkeregisterFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![BosattEtterFregLoven, Doed]);
    }

    #[test]
    fn alle_freg_statuser_med_diplikater_og_case_insensitivitet_gir_freg_fakta() {
        let person = create_person(vec![
            "BOsatTETterFolkeREGisTERLoven",
            "IkkEBOsaTT",
            "doedIFolkeregisteret",
            "forsvunneT",
            "foRSVuNNet",
            "OPPHOERT",
            "oPPHoert",
            "DnuMMer",
        ]);
        let result = UtledeFolkeregisterFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(
            fakta,
            vec![
                BosattEtterFregLoven,
                IkkeBosatt,
                Doed,
                Savnet,
                OpphoertIdentitet,
                Dnummer
            ]
        );
    }
}
