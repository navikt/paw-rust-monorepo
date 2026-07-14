use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct KontortilknytningRow {
    pub id: Uuid,
    pub aktor_id: String,
    pub identitetsnummer: String,
    pub kontor_id: String,
    pub kontor_navn: String,
    pub kontor_type: String,
    pub tidspunkt: DateTime<Utc>,
}

impl KontortilknytningRow {
    pub fn new(
        id: Uuid,
        aktor_id: String,
        identitetsnummer: String,
        kontor_id: String,
        kontor_navn: String,
        kontor_type: String,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt,
        }
    }
}

#[tracing::instrument(skip(tx, id))]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count kontortilknytning by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kontortilknytninger
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip(tx, aktor_id))]
pub async fn select_by_aktor_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    aktor_id: &'a str,
) -> anyhow::Result<Vec<KontortilknytningRow>> {
    tracing::debug!("Select kontortilknytning by aktor_id");
    let rows = sqlx::query_as::<_, KontortilknytningRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt  AT TIME ZONE 'UTC' AS tidspunkt
        FROM kontortilknytninger
        WHERE aktor_id = $1
        "#,
    )
    .bind(aktor_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx, row))]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KontortilknytningRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert kontortilknytning");
    let result = sqlx::query(
        r#"
        INSERT INTO kontortilknytninger (
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&row.id)
    .bind(&row.aktor_id)
    .bind(&row.identitetsnummer)
    .bind(&row.kontor_id)
    .bind(&row.kontor_navn)
    .bind(&row.kontor_type)
    .bind(&row.tidspunkt)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KontortilknytningRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update kontortilknytning");
    let result = sqlx::query(
        r#"
        UPDATE kontortilknytninger SET (
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt
        ) = ($2, $3, $4, $5, $6, $7) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.aktor_id)
    .bind(&row.identitetsnummer)
    .bind(&row.kontor_id)
    .bind(&row.kontor_navn)
    .bind(&row.kontor_type)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, id))]
pub async fn delete<'a>(tx: &mut Transaction<'_, Postgres>, id: &'a Uuid) -> anyhow::Result<u64> {
    tracing::debug!("Delete kontortilknytning");
    let result = sqlx::query(
        r#"
        DELETE FROM kontortilknytninger WHERE id = $1
        "#,
    )
    .bind(&id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
