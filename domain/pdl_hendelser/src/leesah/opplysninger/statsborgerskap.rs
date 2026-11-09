use avro_types::date::EpochDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statsborgerskap {
    pub land: String,
    #[serde_as(as = "Option<EpochDate>")]
    pub gyldig_fom: Option<NaiveDate>,
    #[serde_as(as = "Option<EpochDate>")]
    pub gyldig_tom: Option<NaiveDate>,
    #[serde_as(as = "Option<EpochDate>")]
    pub bekreftelsesdato: Option<NaiveDate>,
}
