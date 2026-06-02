use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AvviksType {
    Forsinkelse,
    #[deprecated(note = "Use SLETTET instead")]
    Retting,
    Slettet,
    TidspunktKorrigert,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}
