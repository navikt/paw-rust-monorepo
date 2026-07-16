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

#[allow(unused)]
#[tracing::instrument(skip(tx, id))]
pub async fn select_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<Option<ProfileringRow>> {
    tracing::debug!("Select profileringer by id");
    let row = sqlx::query_as::<_, ProfileringRow>(
        r#"
        SELECT
            id,
            periode_id,
            opplysninger_id,
            profilert_til,
            tidspunkt AT TIME ZONE 'UTC' AS tidspunkt
        FROM profileringer
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
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
            tidspunkt,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.opplysninger_id)
    .bind(&row.profilert_til)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
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
            tidspunkt,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.opplysninger_id)
    .bind(&row.profilert_til)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
