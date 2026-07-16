use crate::model::dto::kontortilknytning::Kontortilknytning;
use crate::model::dto::ledighetsperiode::Ledighetsperiode;
use serde::Serialize;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arbeidssoeker {
    pub aktor_id: String,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
    pub ledighetsperioder: Vec<Ledighetsperiode>,
    pub kontortilknytninger: Vec<Kontortilknytning>,
}
