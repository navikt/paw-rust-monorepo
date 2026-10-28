use crate::leesah::opplysninger::common::adresse::{
    Matrikkeladresse, UkjentBosted, UtenlandskAdresse, Vegadresse,
};
use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bostedsadresse {
    #[serde_as(as = "Option<AvroDate>")]
    pub angitt_flyttedato: Option<NaiveDate>,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_fra_og_med: Option<NaiveDate>,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_til_og_med: Option<NaiveDate>,
    pub co_adressenavn: Option<String>,
    pub vegadresse: Option<Vegadresse>,
    pub matrikkeladresse: Option<Matrikkeladresse>,
    pub utenlandsk_adresse: Option<UtenlandskAdresse>,
    pub ukjent_bosted: Option<UkjentBosted>,
}
