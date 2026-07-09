use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct PeriodeRow {
    pub id: Uuid,
    pub identitetsnummer: String,
    pub startet_tidspunkt: DateTime<Utc>,
    pub avsluttet_tidspunkt: Option<DateTime<Utc>>,
}

impl PeriodeRow {
    pub fn new(
        id: Uuid,
        identitetsnummer: String,
        startet_tidspunkt: DateTime<Utc>,
        avsluttet_tidspunkt: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            identitetsnummer,
            startet_tidspunkt,
            avsluttet_tidspunkt,
        }
    }
}

#[tracing::instrument(skip(tx))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count periode by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM perioder
        WHERE id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip(tx))]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a PeriodeRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert periode");
    let result = sqlx::query(
        r#"
        INSERT INTO perioder (
            id,
            identitetsnummer,
            startet_tidspunkt,
            avsluttet_tidspunkt
        ) VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&row.id)
    .bind(&row.identitetsnummer)
    .bind(&row.startet_tidspunkt)
    .bind(&row.avsluttet_tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a PeriodeRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update periode");
    let result = sqlx::query(
        r#"
        UPDATE perioder SET (
            identitetsnummer,
            avsluttet_tidspunkt
        ) = ($2, $3) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.identitetsnummer)
    .bind(&row.avsluttet_tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
