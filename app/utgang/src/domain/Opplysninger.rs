use std::collections::HashSet;

use interne_hendelser::vo::Opplysning;

pub struct Opplysninger(pub HashSet<Opplysning>);

impl Opplysninger {
    pub fn er_forhaandsgodkjent(&self) -> bool {
        self.0.contains(&Opplysning::ForhaandsgodkjentAvAnsatt)
    }

    pub fn to_string_vector(&self) -> Vec<String> {
        self.0.iter().map(|o| o.to_string()).collect()
    }
}
