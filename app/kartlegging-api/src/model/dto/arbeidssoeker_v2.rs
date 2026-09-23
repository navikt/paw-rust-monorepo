use crate::model::dto::kontortilknytning::Kontortilknytning;
use crate::model::dto::ledighetsperiode_v2::LedighetsperiodeV2;
use serde::Serialize;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbeidssoekerV2 {
    pub id: i64,
    pub aktor_id: String,
    pub identitetsnummer: String,
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
    pub ledighetsperioder: Vec<LedighetsperiodeV2>,
    pub kontortilknytninger: Vec<Kontortilknytning>,
}

impl ArbeidssoekerV2 {
    pub fn new(
        id: i64,
        aktor_id: String,
        identitetsnummer: String,
        fornavn: Option<String>,
        mellomnavn: Option<String>,
        etternavn: Option<String>,
        ledighetsperioder: Vec<LedighetsperiodeV2>,
        kontortilknytninger: Vec<Kontortilknytning>,
    ) -> Self {
        Self {
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            ledighetsperioder,
            kontortilknytninger,
        }
    }
}
