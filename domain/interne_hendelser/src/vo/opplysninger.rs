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

    pub fn uten_auth_opplysninger(self) -> Self {
        let opplysninger = self
            .0
            .into_iter()
            .filter(|o| !AUTH_OPPLYSNINGER.contains(o))
            .collect();
        Self(opplysninger)
    }
}

const AUTH_OPPLYSNINGER: [Opplysning; 9] = [
    Opplysning::ForhaandsgodkjentAvAnsatt,
    Opplysning::SammeSomInnloggetBruker,
    Opplysning::IkkeSammeSomInnloggerBruker,
    Opplysning::AnsattIkkeTilgang,
    Opplysning::AnsattTilgang,
    Opplysning::IkkeAnsatt,
    Opplysning::SystemIkkeTilgang,
    Opplysning::SystemTilgang,
    Opplysning::IkkeSystem,
];
