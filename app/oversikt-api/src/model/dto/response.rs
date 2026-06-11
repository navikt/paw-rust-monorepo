use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::sort::SortOrder;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OversiktResponse {
    pub arbeidssoekere: Vec<Arbeidssoeker>,
    pub paging: PagingResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagingResponse {
    pub page: i32,
    pub page_size: i32,
    pub total_items: i64,
    pub sort_order: SortOrder,
}
