use crate::model::sort::SortOrder;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeV2Row {
    #[allow(unused)]
    pub arbeidssoeker_id: i64,
    pub periode_id: Uuid,
    pub arbeidssoeker_fra: DateTime<Utc>,
    pub arbeidsledig_fra: Option<DateTime<Utc>>,
    pub egenvurdert_til: Option<String>,
    pub bekreftelse_har_jobbet: Option<bool>,
    pub bekreftelse_vil_fortsette: Option<bool>,
    pub bekreftelse_ansvar: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn select_by_arbeidssoeker_id(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: i64,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<LedighetsperiodeV2Row>> {
    tracing::debug!("Select ledighetsperioder by parent-id");
    let dir = sort_order.as_ref();
    // language=SQL
    let sql = format!(
        r#"
        WITH
        latest_egenvurderinger AS (
            SELECT periode_id, egenvurdert_til
            FROM egenvurderinger
            ORDER BY tidspunkt
            LIMIT 1
        ),
        latest_bekreftelser AS (
            SELECT periode_id, har_jobbet, vil_fortsette, bekreftelsesloesning
            FROM bekreftelser
            ORDER BY tidspunkt
            LIMIT 1
        )
        SELECT
            k.arbeidssoeker_id,
            k.periode_id,
            k.arbeidssoeker_fra AT TIME ZONE 'UTC'                  AS arbeidssoeker_fra,
            k.arbeidsledig_fra AT TIME ZONE 'UTC'                   AS arbeidsledig_fra,
            e.egenvurdert_til,
            b.har_jobbet                                            AS bekreftelse_har_jobbet,
            b.vil_fortsette                                         AS bekreftelse_vil_fortsette,
            COALESCE(bv.bekreftelsesloesninger, ARRAY[]::varchar[]) AS bekreftelse_ansvar
        FROM kartlegginger k
        LEFT JOIN latest_egenvurderinger e    ON e.periode_id  = k.periode_id
        LEFT JOIN latest_bekreftelser b       ON b.periode_id  = k.periode_id
        LEFT JOIN bekreftelse_paavegneav bv   ON bv.periode_id = k.periode_id
        WHERE k.arbeidssoeker_id = $1
        ORDER BY k.arbeidsledig_fra, k.arbeidssoeker_fra {dir}
        OFFSET $2
        LIMIT $3
        "#
    );
    sqlx::query_as::<_, LedighetsperiodeV2Row>(sqlx::AssertSqlSafe(sql))
        .bind(arbeidssoeker_id)
        .bind(offset)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await
        .map_err(Into::into)
}
