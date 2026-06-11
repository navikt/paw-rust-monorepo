use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TilknyttetKontor {
    pub kontor_id: String,
    pub kontor_navn: String,
    pub kontor_type: String,
}
