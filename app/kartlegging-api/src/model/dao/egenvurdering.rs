use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct EgenvurderingRow {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub profilering_id: Uuid,
    pub profilert_til: String,
    pub egenvurdert_til: String,
    pub tidspunkt: DateTime<Utc>,
}

impl EgenvurderingRow {
    pub fn new(
        id: Uuid,
        periode_id: Uuid,
        profilering_id: Uuid,
        profilert_til: String,
        egenvurdert_til: String,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            periode_id,
            profilering_id,
            profilert_til,
            egenvurdert_til,
            tidspunkt,
        }
    }
}

#[tracing::instrument(skip(tx, id))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count egenvurderinger by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM egenvurderinger
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip(tx, row))]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a EgenvurderingRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert egenvurdering");
    let result = sqlx::query(
        r#"
        INSERT INTO egenvurderinger (
            id,
            periode_id,
            profilering_id,
            profilert_til,
            egenvurdert_til,
            tidspunkt
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.profilering_id)
    .bind(&row.profilert_til)
    .bind(&row.egenvurdert_til)
    .bind(&row.tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a EgenvurderingRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update egenvurdering");
    let result = sqlx::query(
        r#"
        UPDATE egenvurderinger SET (
            periode_id,
            profilering_id,
            profilert_til,
            egenvurdert_til,
            tidspunkt
        ) = ($2, $3, $4, $5, $6) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.profilering_id)
    .bind(&row.profilert_til)
    .bind(&row.egenvurdert_til)
    .bind(&row.tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
