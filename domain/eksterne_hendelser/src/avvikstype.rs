use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AvviksType {
    UkjentVerdi,
    Forsinkelse,
    #[deprecated(note = "Use SLETTET instead")]
    Retting,
    Slettet,
    TidspunktKorrigert,
}
