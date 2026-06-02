use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrukerType {
    UkjentVerdi,
    Udefinert,
    Veileder,
    System,
    Sluttbruker,
}

impl Display for BrukerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BrukerType::Sluttbruker => write!(f, "SLUTTBRUKER"),
            BrukerType::Veileder => write!(f, "VEILEDER"),
            BrukerType::System => write!(f, "SYSTEM"),
            BrukerType::Udefinert => write!(f, "UDEFINITERT"),
            BrukerType::UkjentVerdi => write!(f, "UKJENT_VERDI"),
        }
    }
}

impl FromStr for BrukerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SLUTTBRUKER" => Ok(BrukerType::Sluttbruker),
            "VEILEDER" => Ok(BrukerType::Veileder),
            "SYSTEM" => Ok(BrukerType::System),
            "UDEFINITERT" => Ok(BrukerType::Udefinert),
            "UKJENT_VERDI" => Ok(BrukerType::UkjentVerdi),
            _ => Err(format!("Uventet brukertype {}", s)),
        }
    }
}
