use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct ProfileringRow {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub opplysninger_id: Uuid,
    pub profilert_til: String,
    pub tidspunkt: DateTime<Utc>,
}

impl ProfileringRow {
    pub fn new(
        id: Uuid,
        periode_id: Uuid,
        opplysninger_id: Uuid,
        profilert_til: String,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            periode_id,
            opplysninger_id,
            profilert_til,
            tidspunkt,
        }
    }
}

#[tracing::instrument(skip(tx, id))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count profileringer by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM profileringer
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
    row: &'a ProfileringRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert profilering");
    let result = sqlx::query(
        r#"
        INSERT INTO profileringer (
            id,
            periode_id,
            opplysninger_id,
            profilert_til,
            tidspunkt
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.opplysninger_id)
    .bind(&row.profilert_til)
    .bind(&row.tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a ProfileringRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update profilering");
    let result = sqlx::query(
        r#"
        UPDATE profileringer SET (
            periode_id,
            opplysninger_id,
            profilert_til,
            tidspunkt
        ) = ($2, $3, $4, $5) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.opplysninger_id)
    .bind(&row.profilert_til)
    .bind(&row.tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
