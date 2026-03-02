use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrukerType {
    Udefinert,
    UkjentVerdi,
    System,
    Sluttbruker,
    Veileder,
}
