use std::collections::HashSet;

use super::opplysning::Opplysning;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opplysninger(pub HashSet<Opplysning>);

impl Opplysninger {
    pub fn new(opplysninger: Vec<Opplysning>) -> Self {
        Self(opplysninger.into_iter().collect())
    }
    pub fn er_forhaandsgodkjent(&self) -> bool {
        self.0.contains(&Opplysning::ForhaandsgodkjentAvAnsatt)
    }

    pub fn to_string_vector(&self) -> Vec<String> {
        self.0.iter().map(|o| o.to_string()).collect()
    }
}
