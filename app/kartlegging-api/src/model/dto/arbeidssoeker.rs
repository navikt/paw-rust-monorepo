use crate::model::dto::kontortilknytning::Kontortilknytning;
use crate::model::dto::ledighetsperiode::Ledighetsperiode;
use serde::Serialize;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arbeidssoeker {
    pub id: i64,
    pub arbeidssoeker_id: i64, // TODO: Slett etter at frontend er oppdatert til å bruke id
    pub aktor_id: String,
    pub identitetsnummer: String,
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
    pub ledighetsperioder: Vec<Ledighetsperiode>,
    pub kontortilknytninger: Vec<Kontortilknytning>,
}

impl Arbeidssoeker {
    pub fn new(
        id: i64,
        aktor_id: String,
        identitetsnummer: String,
        fornavn: Option<String>,
        mellomnavn: Option<String>,
        etternavn: Option<String>,
        ledighetsperioder: Vec<Ledighetsperiode>,
        kontortilknytninger: Vec<Kontortilknytning>,
    ) -> Self {
        Self {
            id,
            arbeidssoeker_id: id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            ledighetsperioder,
            kontortilknytninger,
        }
    }

    pub fn from_identer(id: i64, aktor_id: String, identitetsnummer: String) -> Self {
        Self {
            id,
            arbeidssoeker_id: id,
            aktor_id,
            identitetsnummer,
            fornavn: None,
            mellomnavn: None,
            etternavn: None,
            ledighetsperioder: Vec::new(),
            kontortilknytninger: Vec::new(),
        }
    }
}
