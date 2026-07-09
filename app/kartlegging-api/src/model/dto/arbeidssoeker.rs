use crate::model::dto::kontortilknytning::Kontortilknytning;
use crate::model::dto::kartlegging::Kartlegging;
use serde::Serialize;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Arbeidssoeker {
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
    pub ledighetsperioder: Vec<Kartlegging>,
    pub kontortilknytninger: Vec<Kontortilknytning>,
}
