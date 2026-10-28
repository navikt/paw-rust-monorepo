use crate::leesah::opplysninger::common::adresse::Koordinater;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrikkeladresse {
    pub matrikkel_id: Option<String>,
    pub bruksenhetsnummer: Option<String>,
    pub tilleggsnavn: Option<String>,
    pub postnummer: Option<String>,
    pub kommunenummer: Option<String>,
    pub koordinater: Option<Koordinater>,
}
