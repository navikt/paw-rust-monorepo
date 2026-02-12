use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AvviksType {
    Forsinkelse,
    #[deprecated(note = "Erstattet av 'SLETTET'")]
    Retting,
    Slettet,
    TidspunktKorrigert,
}
