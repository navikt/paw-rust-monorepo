use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Gradering {
    StrengtFortroligUtland,
    StrengtFortrolig,
    Fortrolig,
    Ugradert,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adressebeskyttelse {
    pub gradering: Gradering,
}
