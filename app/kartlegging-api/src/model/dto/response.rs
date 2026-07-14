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
pub struct StatisticsResponse {
    pub total: i64,
    pub is_null: i64,
    pub is_not_null: i64,
    pub over_30_days: i64,
    pub over_60_days: i64,
    pub over_90_days: i64,
    pub over_180_days: i64,
    pub over_365_days: i64,
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
