use crate::model::dto::arbeidssoeker_v2::ArbeidssoekerV2;
use crate::model::dto::response::PagingResponse;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OversiktResponseV2 {
    pub arbeidssoekere: Vec<ArbeidssoekerV2>,
    pub paging: PagingResponse,
}
