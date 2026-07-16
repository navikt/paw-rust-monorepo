use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct KartleggingRow {
    pub periode_id: Uuid,
    pub parent_id: i64,
    pub arbeidssoeker_siden: DateTime<Utc>,
    pub arbeidsledig_siden: Option<DateTime<Utc>>,
}

impl KartleggingRow {
    pub fn new(
        periode_id: Uuid,
        parent_id: i64,
        arbeidssoeker_siden: DateTime<Utc>,
        arbeidsledig_siden: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            periode_id,
            parent_id,
            arbeidssoeker_siden,
            arbeidsledig_siden,
        }
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct KartleggingMetricsRow {
    pub total: i64,
    pub is_null: i64,
    pub is_not_null: i64,
    pub over_30_days: i64,
    pub over_60_days: i64,
    pub over_90_days: i64,
    pub over_180_days: i64,
    pub over_365_days: i64,
}

#[tracing::instrument(skip(tx))]
pub async fn count_metrics<'a>(
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<KartleggingMetricsRow> {
    tracing::debug!("Count kartlegginger");
    let row = sqlx::query_as::<_, KartleggingMetricsRow>(
        r#"
        SELECT
            COUNT(*)                                                                 AS total,
            COUNT(*) FILTER (WHERE arbeidsledig_siden IS NULL)                       AS is_null,
            COUNT(*) FILTER (WHERE arbeidsledig_siden IS NOT NULL)                   AS is_not_null,
            COUNT(*) FILTER (WHERE arbeidsledig_siden < NOW() - INTERVAL '30 days')  AS over_30_days,
            COUNT(*) FILTER (WHERE arbeidsledig_siden < NOW() - INTERVAL '60 days')  AS over_60_days,
            COUNT(*) FILTER (WHERE arbeidsledig_siden < NOW() - INTERVAL '90 days')  AS over_90_days,
            COUNT(*) FILTER (WHERE arbeidsledig_siden < NOW() - INTERVAL '180 days') AS over_180_days,
            COUNT(*) FILTER (WHERE arbeidsledig_siden < NOW() - INTERVAL '365 days') AS over_365_days
        FROM kartlegginger;
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(row)
}

#[tracing::instrument(skip(tx, periode_id))]
pub async fn select_by_periode_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &'a Uuid,
) -> anyhow::Result<Vec<KartleggingRow>> {
    tracing::debug!("Select kartlegging by periode_id");
    let rows = sqlx::query_as::<_, KartleggingRow>(
        r#"
        SELECT
            periode_id,
            parent_id,
            arbeidssoeker_siden AT TIME ZONE 'UTC' AS arbeidssoeker_siden,
            arbeidsledig_siden  AT TIME ZONE 'UTC' AS arbeidsledig_siden
        FROM kartlegginger
        WHERE periode_id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx, row))]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KartleggingRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert kartlegging");
    let result = sqlx::query(
        r#"
        INSERT INTO kartlegginger (
            periode_id,
            parent_id,
            arbeidssoeker_siden,
            arbeidsledig_siden,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.parent_id)
    .bind(&row.arbeidssoeker_siden)
    .bind(&row.arbeidsledig_siden)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[allow(unused)]
#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KartleggingRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update kartlegging");
    let result = sqlx::query(
        r#"
        UPDATE kartlegginger SET (
            arbeidssoeker_siden,
            arbeidsledig_siden,
            updated_timestamp
        ) = ($2, $3, $4) WHERE periode_id = $1
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.arbeidssoeker_siden)
    .bind(&row.arbeidsledig_siden)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
