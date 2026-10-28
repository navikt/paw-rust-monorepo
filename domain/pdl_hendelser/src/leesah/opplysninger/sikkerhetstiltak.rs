use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kontaktperson {
    pub personident: Option<String>,
    pub norg_enhet: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sikkerhetstiltak {
    pub kontaktperson: Option<Kontaktperson>,
    #[serde(rename = "type")]
    pub tiltakstype: String,
    pub beskrivelse: String,
    #[serde_as(as = "AvroDate")]
    pub gyldig_fom: NaiveDate,
    #[serde_as(as = "AvroDate")]
    pub gyldig_tom: NaiveDate,
}
