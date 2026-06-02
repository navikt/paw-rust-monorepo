use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfilertTil {
    Udefinert,
    AntattGodeMuligheter,
    AntattBehovForVeiledning,
    OppgittHindringer,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}
