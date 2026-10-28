use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statsborgerskap {
    pub land: String,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_fom: Option<NaiveDate>,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_tom: Option<NaiveDate>,
    #[serde_as(as = "Option<AvroDate>")]
    pub bekreftelsesdato: Option<NaiveDate>,
}
