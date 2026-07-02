use crate::model::sort::SortOrder;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, Postgres, Transaction};

#[derive(Debug, FromRow)]
pub(crate) struct ArbeidssoekerRow {
    pub id: i64,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
    pub inserted_timestamp: DateTime<Utc>,
    pub updated_timestamp: Option<DateTime<Utc>>,
}

impl ArbeidssoekerRow {
    pub fn new(
        arbeidssoeker_id: i64,
        identitetsnummer: String,
        fornavn: String,
        mellomnavn: Option<String>,
        etternavn: String,
    ) -> Self {
        Self {
            id: -1,
            arbeidssoeker_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            inserted_timestamp: Utc::now(),
            updated_timestamp: None,
        }
    }
}

#[tracing::instrument(skip(tx))]
pub async fn count_by_identitetsnummer(
    tx: &mut Transaction<'_, Postgres>,
    identitetsnummer: &str,
) -> anyhow::Result<i64> {
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) AS count
        FROM arbeidssoekere a
        WHERE a.identitetsnummer = $1
        "#,
    )
    .bind(identitetsnummer)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip(tx))]
pub async fn count_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &NaiveDate,
) -> anyhow::Result<i64> {
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) AS count
        FROM arbeidssoekere a
        LEFT JOIN ledighetsperioder l on a.id = l.parent_id
        LEFT JOIN kontortilknytninger k on a.id = k.parent_id
        WHERE k.kontor_id = $1 AND k.kontor_type = ANY($2) AND l.ledig_siden NOTNULL AND l.ledig_siden > $3
        "#,
    )
    .bind(kontor_id)
    .bind(&kontor_typer[..])
    .bind(ledig_siden)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip(tx))]
pub async fn select_by_identitetsnummer(
    tx: &mut Transaction<'_, Postgres>,
    identitetsnummer: &str,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    match sort_order {
        SortOrder::Ascending => {
            select_by_identitetsnummer_asc(tx, identitetsnummer, offset, limit).await
        }
        SortOrder::Descending => {
            select_by_identitetsnummer_desc(tx, identitetsnummer, offset, limit).await
        }
    }
}

#[tracing::instrument(skip(tx))]
async fn select_by_identitetsnummer_asc(
    tx: &mut Transaction<'_, Postgres>,
    identitetsnummer: &str,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            a.id,
            a.arbeidssoeker_id,
            a.identitetsnummer,
            a.fornavn,
            a.mellomnavn,
            a.etternavn,
            a.inserted_timestamp AT TIME ZONE 'UTC' AS inserted_timestamp,
            a.updated_timestamp AT TIME ZONE 'UTC' AS updated_timestamp
        FROM arbeidssoekere a
        LEFT JOIN ledighetsperioder l on a.id = l.parent_id
        WHERE a.identitetsnummer = $1
        ORDER BY l.periode_startet
        OFFSET $2
        LIMIT $3
        "#,
    )
    .bind(identitetsnummer)
    .bind(offset)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx))]
async fn select_by_identitetsnummer_desc(
    tx: &mut Transaction<'_, Postgres>,
    identitetsnummer: &str,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            a.id,
            a.arbeidssoeker_id,
            a.identitetsnummer,
            a.fornavn,
            a.mellomnavn,
            a.etternavn,
            a.inserted_timestamp AT TIME ZONE 'UTC' AS inserted_timestamp,
            a.updated_timestamp AT TIME ZONE 'UTC' AS updated_timestamp
        FROM arbeidssoekere a
        LEFT JOIN ledighetsperioder l on a.id = l.parent_id
        WHERE a.identitetsnummer = $1
        ORDER BY l.periode_startet DESC
        OFFSET $2
        LIMIT $3
        "#,
    )
    .bind(identitetsnummer)
    .bind(offset)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx))]
pub async fn select_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &NaiveDate,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    match sort_order {
        SortOrder::Ascending => {
            select_by_kontortilknytning_asc(tx, kontor_id, kontor_typer, ledig_siden, offset, limit)
                .await
        }
        SortOrder::Descending => {
            select_by_kontortilknytning_desc(
                tx,
                kontor_id,
                kontor_typer,
                ledig_siden,
                offset,
                limit,
            )
            .await
        }
    }
}

#[tracing::instrument(skip(tx))]
async fn select_by_kontortilknytning_asc(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &NaiveDate,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            a.id,
            a.arbeidssoeker_id,
            a.identitetsnummer,
            a.fornavn,
            a.mellomnavn,
            a.etternavn,
            a.inserted_timestamp AT TIME ZONE 'UTC' AS inserted_timestamp,
            a.updated_timestamp AT TIME ZONE 'UTC' AS updated_timestamp
        FROM arbeidssoekere a
        LEFT JOIN ledighetsperioder l on a.id = l.parent_id
        LEFT JOIN kontortilknytninger k on a.id = k.parent_id
        WHERE k.kontor_id = $1 AND k.kontor_type = ANY($2) AND l.ledig_siden NOTNULL AND l.ledig_siden > $3
        ORDER BY l.periode_startet
        OFFSET $4
        LIMIT $5
        "#,
    )
    .bind(kontor_id)
    .bind(kontor_typer)
    .bind(ledig_siden)
    .bind(offset)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx))]
async fn select_by_kontortilknytning_desc(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &NaiveDate,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            a.id,
            a.arbeidssoeker_id,
            a.identitetsnummer,
            a.fornavn,
            a.mellomnavn,
            a.etternavn,
            a.inserted_timestamp AT TIME ZONE 'UTC' AS inserted_timestamp,
            a.updated_timestamp AT TIME ZONE 'UTC' AS updated_timestamp
        FROM arbeidssoekere a
        LEFT JOIN ledighetsperioder l on a.id = l.parent_id
        LEFT JOIN kontortilknytninger k on a.id = k.parent_id
        WHERE k.kontor_id = $1 AND k.kontor_type = ANY($2) AND l.ledig_siden NOTNULL AND l.ledig_siden > $3
        ORDER BY l.periode_startet DESC
        OFFSET $4
        LIMIT $5
        "#,
    )
    .bind(kontor_id)
    .bind(kontor_typer)
    .bind(ledig_siden)
    .bind(offset)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx))]
pub async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    row: &ArbeidssoekerRow,
) -> anyhow::Result<i64> {
    let id = sqlx::query_scalar(
        r#"
        INSERT INTO arbeidssoekere (
            arbeidssoeker_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(row.arbeidssoeker_id)
    .bind(&row.identitetsnummer)
    .bind(&row.fornavn)
    .bind(&row.mellomnavn)
    .bind(&row.etternavn)
    .bind(Utc::now())
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

#[tracing::instrument(skip(tx))]
pub async fn update(
    tx: &mut Transaction<'_, Postgres>,
    row: &ArbeidssoekerRow,
) -> anyhow::Result<i64> {
    let id = sqlx::query_scalar(
        r#"
        UPDATE arbeidssoekere SET (
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6) WHERE arbeidssoeker_id = $1
        RETURNING id
        "#,
    )
    .bind(row.arbeidssoeker_id)
    .bind(&row.identitetsnummer)
    .bind(&row.fornavn)
    .bind(&row.mellomnavn)
    .bind(&row.etternavn)
    .bind(Utc::now())
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}
