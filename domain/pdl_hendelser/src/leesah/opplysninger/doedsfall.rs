use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Doedsfall {
    #[serde_as(as = "Option<AvroDate>")]
    pub doedsdato: Option<NaiveDate>,
}
