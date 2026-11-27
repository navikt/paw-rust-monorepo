use crate::model::status::SynkStatus;
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct TiltakSynkRow {
    pub periode_id: Uuid,
    pub synk_id: Uuid,
    pub status: SynkStatus,
    pub status_endret: DateTime<Utc>,
    pub sist_kjoert: DateTime<Utc>,
    pub antall_feil: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct BatchTiltakSynkRow {
    pub periode_id: Uuid,
    pub identitetsnummer: String,
}

#[tracing::instrument(skip_all)]
pub async fn select_by_periode_id(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
) -> anyhow::Result<Option<TiltakSynkRow>> {
    tracing::debug!("Select tiltak_synk");
    let row = sqlx::query_as::<_, TiltakSynkRow>(
        r#"
        SELECT
            periode_id,
            synk_id,
            status,
            status_endret AT TIME ZONE 'UTC' AS status_endret,
            sist_kjoert   AT TIME ZONE 'UTC' AS sist_kjoert,
            antall_feil
        FROM tiltak_synk
        WHERE periode_id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

#[tracing::instrument(skip_all)]
pub async fn count_by_status(
    tx: &mut Transaction<'_, Postgres>,
    status: &SynkStatus,
) -> anyhow::Result<i64> {
    tracing::debug!("Select tiltak_synk");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM tiltak_synk
        WHERE status = $1
        "#,
    )
    .bind(status)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[derive(Debug, sqlx::FromRow)]
pub struct StatusStatistikkRow {
    pub status: SynkStatus,
    pub antall: i64,
    pub vedvarende_feil: i64,
    pub eldste_sist_kjoert: DateTime<Utc>,
}

#[tracing::instrument(skip_all)]
pub async fn select_status_statistikk(
    tx: &mut Transaction<'_, Postgres>,
    vedvarende_feil_grense: i32,
) -> anyhow::Result<Vec<StatusStatistikkRow>> {
    let rows = sqlx::query_as::<_, StatusStatistikkRow>(
        r#"
        SELECT
            status,
            COUNT(*)                                      AS antall,
            COUNT(*) FILTER (WHERE antall_feil >= $1)     AS vedvarende_feil,
            MIN(sist_kjoert) AT TIME ZONE 'UTC'           AS eldste_sist_kjoert
        FROM tiltak_synk
        GROUP BY status
        "#,
    )
    .bind(vedvarende_feil_grense)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    row: &TiltakSynkRow,
) -> anyhow::Result<u64> {
    let result = sqlx::query(
        r#"
        INSERT INTO tiltak_synk (
            periode_id,
            synk_id,
            status,
            status_endret,
            sist_kjoert,
            antall_feil
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.synk_id)
    .bind(&row.status)
    .bind(&row.status_endret)
    .bind(&row.sist_kjoert)
    .bind(&row.antall_feil)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all)]
pub async fn insert_manglende(
    tx: &mut Transaction<'_, Postgres>,
    synk_id: &Uuid,
) -> anyhow::Result<u64> {
    let result = sqlx::query(
        r#"
        INSERT INTO tiltak_synk (periode_id, synk_id, status, status_endret, sist_kjoert, antall_feil)
        SELECT a.periode_id, $1, 'VENTER', timezone('UTC', now()), 'epoch'::timestamp, 0
        FROM arbeidssoekere a
        WHERE NOT EXISTS (SELECT 1 FROM tiltak_synk t WHERE t.periode_id = a.periode_id)
        ON CONFLICT (periode_id) DO NOTHING
        "#,
    )
    .bind(synk_id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[derive(Debug, Clone, PartialEq)]
pub enum Betingelse {
    AntallFeilMinst(i32),
    AntallFeilUnder(i32),
    StatusEndretFoer(DateTime<Utc>),
    SistKjoertFoer(DateTime<Utc>),
    SynkId(Uuid),
}

macro_rules! update_status_sql {
    ($betingelse:literal) => {
        concat!(
            "UPDATE tiltak_synk SET status = $2, status_endret = timezone('UTC', now()), ",
            "antall_feil = CASE WHEN $3 THEN 0 ELSE antall_feil END ",
            "WHERE status = $4 AND ",
            $betingelse
        )
    };
}

#[tracing::instrument(skip_all, fields(fra = %fra, til = %til))]
pub async fn update_status(
    tx: &mut Transaction<'_, Postgres>,
    fra: &SynkStatus,
    til: &SynkStatus,
    betingelse: &Betingelse,
    nullstill_feil: bool,
) -> anyhow::Result<u64> {
    let query = match betingelse {
        Betingelse::AntallFeilMinst(antall) => {
            sqlx::query(update_status_sql!("antall_feil >= $1")).bind(*antall)
        }
        Betingelse::AntallFeilUnder(antall) => {
            sqlx::query(update_status_sql!("antall_feil < $1")).bind(*antall)
        }
        Betingelse::StatusEndretFoer(tidspunkt) => sqlx::query(update_status_sql!(
            "status_endret < ($1 AT TIME ZONE 'UTC')"
        ))
        .bind(*tidspunkt),
        Betingelse::SistKjoertFoer(tidspunkt) => {
            sqlx::query(update_status_sql!("sist_kjoert < ($1 AT TIME ZONE 'UTC')"))
                .bind(*tidspunkt)
        }
        Betingelse::SynkId(synk_id) => {
            sqlx::query(update_status_sql!("synk_id = $1")).bind(*synk_id)
        }
    };
    let result = query
        .bind(til)
        .bind(nullstill_feil)
        .bind(fra)
        .execute(&mut **tx)
        .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all, fields(periode_id = %periode_id, til = %til))]
pub async fn update_resultat(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
    synk_id: &Uuid,
    til: &SynkStatus,
) -> anyhow::Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE tiltak_synk
        SET status        = $3,
            status_endret = timezone('UTC', now()),
            sist_kjoert   = timezone('UTC', now()),
            antall_feil   = CASE WHEN $3 = 'FERDIG' THEN 0 ELSE antall_feil + 1 END
        WHERE periode_id = $1 AND synk_id = $2 AND status = 'AKTIV'
        "#,
    )
    .bind(periode_id)
    .bind(synk_id)
    .bind(til)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all)]
pub async fn select_batch(
    tx: &mut Transaction<'_, Postgres>,
    synk_id: &Uuid,
    batch_stoerrelse: i64,
) -> anyhow::Result<Vec<BatchTiltakSynkRow>> {
    let result = sqlx::query_as::<_, BatchTiltakSynkRow>(
        r#"
        WITH kandidater AS (
            SELECT periode_id FROM tiltak_synk
            WHERE status = 'VENTER'
            ORDER BY sist_kjoert, periode_id
            LIMIT $2
            FOR UPDATE SKIP LOCKED
        )
        UPDATE tiltak_synk t
        SET status = 'AKTIV', synk_id = $1, status_endret = timezone('UTC', now())
        FROM kandidater k, arbeidssoekere a
        WHERE t.periode_id = k.periode_id
          AND a.periode_id = t.periode_id
          AND t.status = 'VENTER'
        RETURNING t.periode_id, a.identitetsnummer
        "#,
    )
    .bind(synk_id)
    .bind(batch_stoerrelse)
    .fetch_all(&mut **tx)
    .await;

    match result {
        Ok(rows) => Ok(rows),
        Err(error) if er_kollisjon(&error) => {
            tracing::debug!("Claim kolliderte med annen instans: {error}");
            Ok(Vec::new())
        }
        Err(error) => Err(error.into()),
    }
}

const SERIALIZATION_FAILURE: &str = "40001";
const DEADLOCK_DETECTED: &str = "40P01";

fn er_kollisjon(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .is_some_and(|code| code == SERIALIZATION_FAILURE || code == DEADLOCK_DETECTED)
}
