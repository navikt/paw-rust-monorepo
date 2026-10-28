use crate::leesah::opplysninger::common::adresse::Koordinater;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vegadresse {
    pub matrikkel_id: Option<String>,
    pub husnummer: Option<String>,
    pub husbokstav: Option<String>,
    pub bruksenhetsnummer: Option<String>,
    pub adressenavn: Option<String>,
    pub kommunenummer: Option<String>,
    pub bydelsnummer: Option<String>,
    pub tilleggsnavn: Option<String>,
    pub postnummer: Option<String>,
    pub koordinater: Option<Koordinater>,
}
