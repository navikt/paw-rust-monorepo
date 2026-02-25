use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct PeriodeMetadata {
    periode_id: Uuid,
    identitetsnummer: String,
    arbeidssoeker_id: i64,
    kafka_key: i64,
}
