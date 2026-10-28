use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sivilstand {
    #[serde(rename = "type")]
    pub sivilstand_type: String,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_fra_og_med: Option<NaiveDate>,
    pub relatert_ved_sivilstand: Option<String>,
    #[serde_as(as = "Option<AvroDate>")]
    pub bekreftelsesdato: Option<NaiveDate>,
}
