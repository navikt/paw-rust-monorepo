use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::sort::SortOrder;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KartleggingResponse {
    pub arbeidssoekere: Vec<Arbeidssoeker>,
    pub paging: PagingResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagingResponse {
    pub page: i32,
    pub page_size: i32,
    pub hit_size: i32,
    pub total_count: i64,
    pub sort_order: SortOrder,
}
