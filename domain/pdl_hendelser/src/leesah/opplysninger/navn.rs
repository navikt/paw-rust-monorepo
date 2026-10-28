use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginaltNavn {
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Navn {
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
    pub forkortet_navn: Option<String>,
    pub originalt_navn: Option<OriginaltNavn>,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_fra_og_med: Option<NaiveDate>,
}
