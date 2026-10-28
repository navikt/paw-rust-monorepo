use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Foedsel {
    pub foedselsaar: Option<i32>,
    #[serde_as(as = "Option<AvroDate>")]
    pub foedselsdato: Option<NaiveDate>,
    pub foedeland: Option<String>,
    pub foedested: Option<String>,
    pub foedekommune: Option<String>,
}
