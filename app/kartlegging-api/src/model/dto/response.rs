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
    pub over_0030_days: i64,
    pub over_0060_days: i64,
    pub over_0090_days: i64,
    pub over_0180_days: i64,
    pub over_0365_days: i64,
    pub over_0730_days: i64,
    pub over_1095_days: i64,
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
