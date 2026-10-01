use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct KartleggingRow {
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    pub arbeidssoeker_fra: DateTime<Utc>,
    pub arbeidssoeker_til: Option<DateTime<Utc>>,
    pub arbeidsledig_fra: Option<DateTime<Utc>>,
}

impl KartleggingRow {
    pub fn new(
        periode_id: Uuid,
        arbeidssoeker_id: i64,
        arbeidssoeker_fra: DateTime<Utc>,
        arbeidssoeker_til: Option<DateTime<Utc>>,
        arbeidsledig_fra: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            periode_id,
            arbeidssoeker_id,
            arbeidssoeker_fra,
            arbeidssoeker_til,
            arbeidsledig_fra,
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn select_by_periode_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &'a Uuid,
) -> anyhow::Result<Vec<KartleggingRow>> {
    tracing::debug!("Select kartlegging by periode_id");
    let rows = sqlx::query_as::<_, KartleggingRow>(
        r#"
        SELECT
            periode_id,
            arbeidssoeker_id,
            arbeidssoeker_fra AT TIME ZONE 'UTC' AS arbeidssoeker_fra,
            arbeidssoeker_til AT TIME ZONE 'UTC' AS arbeidssoeker_til,
            arbeidsledig_fra  AT TIME ZONE 'UTC' AS arbeidsledig_fra
        FROM kartlegginger
        WHERE periode_id = $1
        ORDER BY arbeidssoeker_fra DESC
        "#,
    )
    .bind(periode_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_arbeidssoeker_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: &'a i64,
) -> anyhow::Result<Vec<KartleggingRow>> {
    tracing::debug!("Select latest kartlegging by arbeidssoeker_id");
    let rows = sqlx::query_as::<_, KartleggingRow>(
        r#"
        SELECT
            periode_id,
            arbeidssoeker_id,
            arbeidssoeker_fra AT TIME ZONE 'UTC' AS arbeidssoeker_fra,
            arbeidssoeker_til AT TIME ZONE 'UTC' AS arbeidssoeker_til,
            arbeidsledig_fra  AT TIME ZONE 'UTC' AS arbeidsledig_fra
        FROM kartlegginger
        WHERE arbeidssoeker_id = $1
        ORDER BY arbeidssoeker_fra DESC
        "#,
    )
    .bind(arbeidssoeker_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KartleggingRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert kartlegging");
    let result = sqlx::query(
        r#"
        INSERT INTO kartlegginger (
            periode_id,
            arbeidssoeker_id,
            arbeidssoeker_fra,
            arbeidssoeker_til,
            arbeidsledig_fra,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.arbeidssoeker_id)
    .bind(&row.arbeidssoeker_fra)
    .bind(&row.arbeidssoeker_til)
    .bind(&row.arbeidsledig_fra)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[allow(unused)]
#[tracing::instrument(skip_all)]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &'a Uuid,
    arbeidssoeker_til: &'a Option<DateTime<Utc>>,
    arbeidsledig_fra: &'a Option<DateTime<Utc>>,
) -> anyhow::Result<u64> {
    tracing::debug!("Update kartlegging");
    let result = sqlx::query(
        r#"
        UPDATE kartlegginger SET (
            arbeidssoeker_til,
            arbeidsledig_fra,
            updated_timestamp
        ) = ($2, $3, $4) WHERE periode_id = $1
        "#,
    )
    .bind(periode_id)
    .bind(arbeidssoeker_til)
    .bind(arbeidsledig_fra)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
