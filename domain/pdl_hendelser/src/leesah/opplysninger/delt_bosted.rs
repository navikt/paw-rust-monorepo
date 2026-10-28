use crate::leesah::opplysninger::common::adresse::{Matrikkeladresse, UkjentBosted, Vegadresse};
use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltBosted {
    #[serde_as(as = "AvroDate")]
    pub startdato_for_kontrakt: NaiveDate,
    #[serde_as(as = "Option<AvroDate>")]
    pub sluttdato_for_kontrakt: Option<NaiveDate>,
    pub co_adressenavn: Option<String>,
    pub vegadresse: Option<Vegadresse>,
    pub matrikkeladresse: Option<Matrikkeladresse>,
    pub ukjent_bosted: Option<UkjentBosted>,
}
