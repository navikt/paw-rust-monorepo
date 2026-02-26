use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct PeriodeMetadata {
    pub periode_id: Uuid,
    pub identitetsnummer: String,
    pub arbeidssoeker_id: i64,
    pub kafka_key: i64,
}
