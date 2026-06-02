use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdentitetType {
    Folkeregisterident,
    Aktorid,
    Npid,
    Arbeidssoekerid,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}
