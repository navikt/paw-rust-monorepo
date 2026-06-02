use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfilertTil {
    UkjentVerdi,
    Udefinert,
    AntattGodeMuligheter,
    AntattBehovForVeiledning,
    OppgittHindringer,
}
