use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrukerType {
    Udefinert,
    UkjentVerdi,
    System,
    Sluttbruker,
    Veileder,
}
