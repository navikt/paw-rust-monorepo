use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtflyttingFraNorge {
    pub tilflyttingsland: Option<String>,
    pub tilflyttingssted_i_utlandet: Option<String>,
    #[serde_as(as = "Option<AvroDate>")]
    pub utflyttingsdato: Option<NaiveDate>,
}
