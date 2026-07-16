use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct BekreftelseRow {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub gjelder_fra: DateTime<Utc>,
    pub gjelder_til: DateTime<Utc>,
    pub har_jobbet: bool,
    pub vil_fortsette: bool,
    pub bekreftelsesloesning: String,
    pub tidspunkt: DateTime<Utc>,
}

impl BekreftelseRow {
    pub fn new(
        id: Uuid,
        periode_id: Uuid,
        gjelder_fra: DateTime<Utc>,
        gjelder_til: DateTime<Utc>,
        har_jobbet: bool,
        vil_fortsette: bool,
        bekreftelsesloesning: String,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            periode_id,
            gjelder_fra,
            gjelder_til,
            har_jobbet,
            vil_fortsette,
            bekreftelsesloesning,
            tidspunkt,
        }
    }
}

#[tracing::instrument(skip(tx, id))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count bekreftelser by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM bekreftelser
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
) -> anyhow::Result<Option<BekreftelseRow>> {
    tracing::debug!("Select bekreftelse by id");
    let row = sqlx::query_as::<_, BekreftelseRow>(
        r#"
        SELECT
            id,
            periode_id,
            gjelder_fra AT TIME ZONE 'UTC' AS gjelder_fra,
            gjelder_til AT TIME ZONE 'UTC' AS gjelder_til,
            har_jobbet,
            vil_fortsette,
            bekreftelsesloesning,
            tidspunkt   AT TIME ZONE 'UTC' AS tidspunkt
        FROM bekreftelser
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
    row: &'a BekreftelseRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert bekreftelse");
    let result = sqlx::query(
        r#"
        INSERT INTO bekreftelser (
            id,
            periode_id,
            gjelder_fra,
            gjelder_til,
            har_jobbet,
            vil_fortsette,
            bekreftelsesloesning,
            tidspunkt,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.gjelder_fra)
    .bind(&row.gjelder_til)
    .bind(&row.har_jobbet)
    .bind(&row.vil_fortsette)
    .bind(&row.bekreftelsesloesning)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a BekreftelseRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update bekreftelse");
    let result = sqlx::query(
        r#"
        UPDATE bekreftelser SET (
            periode_id,
            gjelder_fra,
            gjelder_til,
            har_jobbet,
            vil_fortsette,
            bekreftelsesloesning,
            tidspunkt,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6, $7, $8, $9) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.gjelder_fra)
    .bind(&row.gjelder_til)
    .bind(&row.har_jobbet)
    .bind(&row.vil_fortsette)
    .bind(&row.bekreftelsesloesning)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
