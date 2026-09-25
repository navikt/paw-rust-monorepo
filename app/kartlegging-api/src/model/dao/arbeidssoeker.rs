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

#[tracing::instrument(skip_all)]
pub async fn count_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &Option<NaiveDate>,
) -> anyhow::Result<i64> {
    tracing::debug!("Count arbeidssøkere by kontortilknytning");
    let count = match ledig_siden {
        Some(timestamp) => {
            sqlx::query_scalar(
                r#"
            SELECT COUNT(DISTINCT a.id) AS count
            FROM arbeidssoekere a
            LEFT JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            LEFT JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
            "#,
            ).bind(kontor_id)
                    .bind(&kontor_typer[..])
                    .bind(timestamp)
                    .fetch_one(&mut **tx)
                    .await?
        }
        None => sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT a.id) AS count
            FROM arbeidssoekere a
            LEFT JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            LEFT JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2)
            "#,
        ).bind(kontor_id)
                .bind(&kontor_typer[..])
                .fetch_one(&mut **tx)
                .await?,
    };
    Ok(count)
}

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip_all)]
pub async fn select_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &Option<NaiveDate>,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    tracing::debug!("Select arbeidssøkere by kontortilknytning");
    let dir = sort_order.as_ref();
    let rows = match ledig_siden {
        None => {
            let sql = format!(
                r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM (
            SELECT DISTINCT ON (a.id)
                a.id,
                a.aktor_id,
                a.identitetsnummer,
                a.fornavn,
                a.mellomnavn,
                a.etternavn,
                k.arbeidsledig_fra AS sort_arbeidsledig_fra,
                k.arbeidssoeker_fra AS sort_arbeidssoeker_fra
            FROM arbeidssoekere a
            LEFT JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            LEFT JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2)
        ) distinct_arbeidssoekere
        ORDER BY sort_arbeidsledig_fra, sort_arbeidssoeker_fra {dir}
        OFFSET $3
        LIMIT $4
        "#,
            );
            sqlx::query_as::<_, ArbeidssoekerRow>(sqlx::AssertSqlSafe(sql))
                .bind(kontor_id)
                .bind(kontor_typer)
                .bind(offset)
                .bind(limit)
                .fetch_all(&mut **tx)
                .await?
        }
        Some(timestamp) => {
            let sql = format!(
                r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM (
            SELECT DISTINCT ON (a.id)
                a.id,
                a.aktor_id,
                a.identitetsnummer,
                a.fornavn,
                a.mellomnavn,
                a.etternavn,
                k.arbeidsledig_fra AS sort_arbeidsledig_fra,
                k.arbeidssoeker_fra AS sort_arbeidssoeker_fra
            FROM arbeidssoekere a
            LEFT JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            LEFT JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
        ) distinct_arbeidssoekere
        ORDER BY sort_arbeidsledig_fra, sort_arbeidssoeker_fra {dir}
        OFFSET $4
        LIMIT $5
        "#,
            );
            sqlx::query_as::<_, ArbeidssoekerRow>(sqlx::AssertSqlSafe(sql))
                .bind(kontor_id)
                .bind(kontor_typer)
                .bind(timestamp)
                .bind(offset)
                .bind(limit)
                .fetch_all(&mut **tx)
                .await?
        }
    };
    Ok(rows)
}

#[tracing::instrument(skip_all)]
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
#[tracing::instrument(skip_all)]
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
