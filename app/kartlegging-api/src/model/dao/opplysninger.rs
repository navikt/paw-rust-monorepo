use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct OpplysningerRow {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub jobbsituasjon: Vec<String>,
    pub tidspunkt: DateTime<Utc>,
}

impl OpplysningerRow {
    pub fn new(
        id: Uuid,
        periode_id: Uuid,
        jobbsituasjon: Vec<String>,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            periode_id,
            jobbsituasjon,
            tidspunkt,
        }
    }
}

#[tracing::instrument(skip(tx, id))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count opplysninger by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM opplysninger
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
) -> anyhow::Result<Option<OpplysningerRow>> {
    tracing::debug!("Select opplysninger by id");
    let row = sqlx::query_as::<_, OpplysningerRow>(
        r#"
        SELECT
            id,
            periode_id,
            jobbsituasjon,
            tidspunkt AT TIME ZONE 'UTC' AS tidspunkt
        FROM opplysninger
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
    row: &'a OpplysningerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert opplysninger");
    let result = sqlx::query(
        r#"
        INSERT INTO opplysninger (
            id,
            periode_id,
            jobbsituasjon,
            tidspunkt,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.jobbsituasjon)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a OpplysningerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update opplysninger");
    let result = sqlx::query(
        r#"
        UPDATE opplysninger SET (
            periode_id,
            jobbsituasjon,
            tidspunkt,
            updated_timestamp
        ) = ($2, $3, $4, $5) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.periode_id)
    .bind(&row.jobbsituasjon)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
