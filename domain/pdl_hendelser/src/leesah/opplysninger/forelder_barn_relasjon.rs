use crate::leesah::opplysninger::common::{KjoennType, Personnavn};
use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForelderBarnRelasjon {
    pub relatert_persons_ident: Option<String>,
    pub relatert_persons_rolle: String,
    pub min_rolle_for_person: Option<String>,
    pub relatert_person_uten_folkeregisteridentifikator: Option<RelatertBiPerson>,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatertBiPerson {
    pub navn: Personnavn,
    #[serde_as(as = "Option<AvroDate>")]
    pub foedselsdato: Option<NaiveDate>,
    pub statsborgerskap: Option<String>,
    pub kjoenn: Option<KjoennType>,
}
