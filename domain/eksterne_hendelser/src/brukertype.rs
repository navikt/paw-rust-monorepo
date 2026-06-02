use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrukerType {
    Udefinert,
    Veileder,
    System,
    Sluttbruker,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

impl Display for BrukerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BrukerType::Sluttbruker => write!(f, "SLUTTBRUKER"),
            BrukerType::Veileder => write!(f, "VEILEDER"),
            BrukerType::System => write!(f, "SYSTEM"),
            BrukerType::Udefinert => write!(f, "UDEFINITERT"),
            BrukerType::UkjentVerdi => write!(f, "UkjentVerdi"),
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
            "UkjentVerdi" => Ok(BrukerType::UkjentVerdi),
            _ => Err(format!("Uventet brukertype {}", s)),
        }
    }
}
