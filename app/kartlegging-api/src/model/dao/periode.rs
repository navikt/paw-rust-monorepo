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

#[tracing::instrument(skip(tx, id))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count perioder by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM perioder
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
) -> anyhow::Result<Option<PeriodeRow>> {
    tracing::debug!("Select periode by id");
    let row = sqlx::query_as::<_, PeriodeRow>(
        r#"
        SELECT
            id,
            identitetsnummer,
            startet_tidspunkt   AT TIME ZONE 'UTC' AS startet_tidspunkt,
            avsluttet_tidspunkt AT TIME ZONE 'UTC' AS avsluttet_tidspunkt
        FROM perioder
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
    row: &'a PeriodeRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert periode");
    let result = sqlx::query(
        r#"
        INSERT INTO perioder (
            id,
            identitetsnummer,
            startet_tidspunkt,
            avsluttet_tidspunkt,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&row.id)
    .bind(&row.identitetsnummer)
    .bind(&row.startet_tidspunkt)
    .bind(&row.avsluttet_tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a PeriodeRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update periode");
    let result = sqlx::query(
        r#"
        UPDATE perioder SET (
            identitetsnummer,
            avsluttet_tidspunkt,
            updated_timestamp
        ) = ($2, $3, $4) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.identitetsnummer)
    .bind(&row.avsluttet_tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
