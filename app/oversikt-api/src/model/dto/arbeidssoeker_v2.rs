use crate::model::dto::kontor::TilknyttetKontor;
use crate::model::dto::ledighetsperiode::Ledighetsperiode;
use serde::Serialize;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbeidssoekerV2 {
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
    pub ledighetsperioder: Vec<Ledighetsperiode>,
    pub tilknyttet_kontor: Vec<TilknyttetKontor>,
}
