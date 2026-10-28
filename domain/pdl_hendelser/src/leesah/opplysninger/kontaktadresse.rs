use crate::leesah::opplysninger::common::adresse::{
    PostadresseIFrittFormat, Postboksadresse, UtenlandskAdresse, UtenlandskAdresseIFrittFormat,
    Vegadresse,
};
use crate::serde::AvroDate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kontaktadresse {
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_fra_og_med: Option<NaiveDate>,
    #[serde_as(as = "Option<AvroDate>")]
    pub gyldig_til_og_med: Option<NaiveDate>,
    #[serde(rename = "type")]
    pub kontaktadresse_type: String,
    pub co_adressenavn: Option<String>,
    pub postboksadresse: Option<Postboksadresse>,
    pub vegadresse: Option<Vegadresse>,
    pub postadresse_i_fritt_format: Option<PostadresseIFrittFormat>,
    pub utenlandsk_adresse: Option<UtenlandskAdresse>,
    pub utenlandsk_adresse_i_fritt_format: Option<UtenlandskAdresseIFrittFormat>,
}
