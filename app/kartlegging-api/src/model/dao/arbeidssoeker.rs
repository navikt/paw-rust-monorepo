use crate::model::sort::SortOrder;
use chrono::{NaiveDate, Utc};
use sqlx::{FromRow, Postgres, Transaction};

#[derive(Debug, FromRow)]
pub(crate) struct ArbeidssoekerRow {
    pub id: i64,
    pub aktor_id: String,
    pub identitetsnummer: String,
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
}

impl ArbeidssoekerRow {
    pub fn new(
        id: i64,
        aktor_id: String,
        identitetsnummer: String,
        fornavn: Option<String>,
        mellomnavn: Option<String>,
        etternavn: Option<String>,
    ) -> Self {
        Self {
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
        }
    }
}

#[tracing::instrument(skip(tx))]
pub async fn count_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &NaiveDate,
) -> anyhow::Result<i64> {
    tracing::debug!("Count arbeidssøkere by kontortilknytning");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) AS count
        FROM arbeidssoekere a
        LEFT JOIN kartlegginger k on a.id = k.arbeidssoeker_id
        LEFT JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
        WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
        "#,
    )
    .bind(kontor_id)
    .bind(&kontor_typer[..])
    .bind(ledig_siden)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[tracing::instrument(skip(tx, arbeidssoeker_id))]
pub async fn select_by_arbeidssoeker_id(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: &i64,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    tracing::debug!("Select arbeidssøkere by arbeidssoeker_id");
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM arbeidssoekere
        WHERE id = $1
        "#,
    )
    .bind(arbeidssoeker_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx, identitetsnummer))]
pub async fn select_by_identitetsnummer(
    tx: &mut Transaction<'_, Postgres>,
    identitetsnummer: &str,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    tracing::debug!("Select arbeidssøkere by identitetsnummer");
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM arbeidssoekere
        WHERE identitetsnummer = $1
        "#,
    )
    .bind(identitetsnummer)
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
    tracing::debug!("Select arbeidssøkere by kontortilknytning");
    let dir = sort_order.as_ref();
    // language=SQL
    let sql = format!(
        r#"
        SELECT
            a.id,
            a.aktor_id,
            a.identitetsnummer,
            a.fornavn,
            a.mellomnavn,
            a.etternavn
        FROM arbeidssoekere a
        LEFT JOIN kartlegginger k on a.id = k.arbeidssoeker_id
        LEFT JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
        WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
        ORDER BY k.arbeidssoeker_fra {}
        OFFSET $4
        LIMIT $5
        "#,
        dir
    );
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(sqlx::AssertSqlSafe(sql))
        .bind(kontor_id)
        .bind(kontor_typer)
        .bind(ledig_siden)
        .bind(offset)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx, row))]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a ArbeidssoekerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
    let result = sqlx::query(
        r#"
        INSERT INTO arbeidssoekere (
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&row.id)
    .bind(&row.aktor_id)
    .bind(&row.identitetsnummer)
    .bind(&row.fornavn)
    .bind(&row.mellomnavn)
    .bind(&row.etternavn)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[allow(unused)]
#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a ArbeidssoekerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update arbeidssøker");
    let result = sqlx::query(
        r#"
        UPDATE arbeidssoekere SET (
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6) WHERE id = $1
        "#,
    )
    .bind(row.id)
    .bind(&row.identitetsnummer)
    .bind(&row.fornavn)
    .bind(&row.mellomnavn)
    .bind(&row.etternavn)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
