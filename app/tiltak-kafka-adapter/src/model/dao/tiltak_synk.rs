use crate::model::status::SynkStatus;
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct TiltakSynkRow {
    pub periode_id: Uuid,
    pub synk_id: Uuid,
    pub status: SynkStatus,
    pub status_endret: DateTime<Utc>,
    pub sist_kjoert: DateTime<Utc>,
    pub antall_feil: i32,
}

impl TiltakSynkRow {
    pub fn new(
        periode_id: Uuid,
        synk_id: Uuid,
        status: SynkStatus,
        status_endret: DateTime<Utc>,
        sist_kjoert: DateTime<Utc>,
        antall_feil: i32,
    ) -> Self {
        Self {
            periode_id,
            synk_id,
            status,
            status_endret,
            sist_kjoert,
            antall_feil,
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a TiltakSynkRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
    let result = sqlx::query(
        r#"
        INSERT INTO tiltak_synk (
            periode_id,
            synk_id,
            status,
            status_endret,
            sist_kjoert,
            antall_feil
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.synk_id)
    .bind(&row.status)
    .bind(&row.status_endret)
    .bind(&row.sist_kjoert)
    .bind(&row.antall_feil)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
