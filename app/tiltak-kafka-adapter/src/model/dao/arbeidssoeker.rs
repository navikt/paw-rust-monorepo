use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct ArbeidssoekerRow {
    pub periode_id: Uuid,
    pub identitetsnummer: String,
    pub periode_startet: DateTime<Utc>,
}

#[tracing::instrument(skip_all)]
pub async fn count(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<i64> {
    tracing::debug!("Count arbeidssøkere");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM arbeidssoekere
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip_all)]
pub async fn count_by_periode_id(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count arbeidssøkere");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM arbeidssoekere
        WHERE periode_id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip_all)]
pub async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    row: &ArbeidssoekerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
    let result = sqlx::query(
        r#"
        INSERT INTO arbeidssoekere (
            periode_id,
            identitetsnummer,
            periode_startet
        ) VALUES ($1, $2, $3)
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.identitetsnummer)
    .bind(&row.periode_startet)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all)]
pub async fn update(
    tx: &mut Transaction<'_, Postgres>,
    row: &ArbeidssoekerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
    let result = sqlx::query(
        r#"
        UPDATE arbeidssoekere SET (
            identitetsnummer,
            periode_startet
        ) = ($2, $3)
        WHERE periode_id = $1
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.identitetsnummer)
    .bind(&row.periode_startet)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all)]
pub async fn delete_by_periode_id(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
    let result = sqlx::query(
        r#"
        DELETE FROM arbeidssoekere
        WHERE periode_id = $1
        "#,
    )
    .bind(&periode_id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
