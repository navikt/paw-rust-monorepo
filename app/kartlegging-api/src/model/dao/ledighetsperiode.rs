use crate::model::sort::SortOrder;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeRow {
    pub parent_id: i64,
    pub periode_id: Uuid,
    pub arbeidssoeker_siden: DateTime<Utc>,
    pub arbeidsledig_siden: Option<DateTime<Utc>>,
    pub periode_startet: DateTime<Utc>,
    pub periode_avsluttet: Option<DateTime<Utc>>,
    pub opplysninger_id: Option<Uuid>,
    pub opplysninger_jobbsituasjon: Vec<String>,
    pub opplysninger_tidspunkt: Option<DateTime<Utc>>,
    pub profilering_id: Option<Uuid>,
    pub profilert_til: Option<String>,
    pub profilering_tidspunkt: Option<DateTime<Utc>>,
    pub egenvurdering_id: Option<Uuid>,
    pub egenvurdert_til: Option<String>,
    pub egenvurdering_tidspunkt: Option<DateTime<Utc>>,
    pub bekreftelse_id: Option<Uuid>,
    pub bekreftelse_gjelder_fra: Option<DateTime<Utc>>,
    pub bekreftelse_gjelder_til: Option<DateTime<Utc>>,
    pub bekreftelse_har_jobbet: Option<bool>,
    pub bekreftelse_vil_fortsette: Option<bool>,
    pub bekreftelsesloesning: Option<String>,
    pub bekreftelse_paa_vegne_av: Vec<String>,
}

impl LedighetsperiodeRow {
    pub fn new(
        parent_id: i64,
        periode_id: Uuid,
        periode_startet: DateTime<Utc>,
        periode_avsluttet: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            parent_id,
            periode_id,
            arbeidssoeker_siden: periode_startet,
            arbeidsledig_siden: None,
            periode_startet,
            periode_avsluttet,
            opplysninger_id: None,
            opplysninger_jobbsituasjon: Vec::new(),
            opplysninger_tidspunkt: None,
            profilering_id: None,
            profilert_til: None,
            profilering_tidspunkt: None,
            egenvurdering_id: None,
            egenvurdert_til: None,
            egenvurdering_tidspunkt: None,
            bekreftelse_id: None,
            bekreftelse_gjelder_fra: None,
            bekreftelse_gjelder_til: None,
            bekreftelse_har_jobbet: None,
            bekreftelse_vil_fortsette: None,
            bekreftelsesloesning: None,
            bekreftelse_paa_vegne_av: Vec::new(),
        }
    }
}

#[tracing::instrument(skip(tx, parent_id))]
pub async fn select_by_parent_id(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: i64,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<LedighetsperiodeRow>> {
    tracing::debug!("Select ledighetsperioder by parent-id");
    let dir = sort_order.as_ref();
    // language=SQL
    let sql = format!(
        r#"
        WITH latest_opplysninger AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, jobbsituasjon, tidspunkt
            FROM opplysninger
            ORDER BY periode_id, tidspunkt {dir}
        ),
        latest_profileringer AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, profilert_til, tidspunkt
            FROM profileringer
            ORDER BY periode_id, tidspunkt {dir}
        ),
        latest_egenvurderinger AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, egenvurdert_til, tidspunkt
            FROM egenvurderinger
            ORDER BY periode_id, tidspunkt {dir}
        ),
        latest_bekreftelser AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, gjelder_fra, gjelder_til,
                   har_jobbet, vil_fortsette, bekreftelsesloesning
            FROM bekreftelser
            ORDER BY periode_id, tidspunkt {dir}
        )
        SELECT
            k.parent_id,
            k.periode_id,
            k.arbeidssoeker_siden AT TIME ZONE 'UTC'                AS arbeidssoeker_siden,
            k.arbeidsledig_siden AT TIME ZONE 'UTC'                 AS arbeidsledig_siden,
            p.startet_tidspunkt AT TIME ZONE 'UTC'                  AS periode_startet,
            p.avsluttet_tidspunkt AT TIME ZONE 'UTC'                AS periode_avsluttet,
            o.id                                                    AS opplysninger_id,
            COALESCE(o.jobbsituasjon, ARRAY[]::varchar[])           AS opplysninger_jobbsituasjon,
            o.tidspunkt AT TIME ZONE 'UTC'                          AS opplysninger_tidspunkt,
            pr.id                                                   AS profilering_id,
            pr.profilert_til,
            pr.tidspunkt AT TIME ZONE 'UTC'                         AS profilering_tidspunkt,
            e.id                                                    AS egenvurdering_id,
            e.egenvurdert_til,
            e.tidspunkt AT TIME ZONE 'UTC'                          AS egenvurdering_tidspunkt,
            b.id                                                    AS bekreftelse_id,
            b.gjelder_fra AT TIME ZONE 'UTC'                        AS bekreftelse_gjelder_fra,
            b.gjelder_til AT TIME ZONE 'UTC'                        AS bekreftelse_gjelder_til,
            b.har_jobbet                                            AS bekreftelse_har_jobbet,
            b.vil_fortsette                                         AS bekreftelse_vil_fortsette,
            b.bekreftelsesloesning,
            COALESCE(bv.bekreftelsesloesninger, ARRAY[]::varchar[]) AS bekreftelse_paa_vegne_av
        FROM kartlegginger k
        LEFT JOIN perioder p                  ON p.id          = k.periode_id
        LEFT JOIN latest_opplysninger o       ON o.periode_id  = k.periode_id
        LEFT JOIN latest_profileringer pr     ON pr.periode_id = k.periode_id
        LEFT JOIN latest_egenvurderinger e    ON e.periode_id  = k.periode_id
        LEFT JOIN latest_bekreftelser b       ON b.periode_id  = k.periode_id
        LEFT JOIN bekreftelse_paavegneav bv ON bv.periode_id = k.periode_id
        WHERE k.parent_id = $1
        ORDER BY k.arbeidssoeker_siden {dir}
        OFFSET $2
        LIMIT $3
        "#
    );
    sqlx::query_as::<_, LedighetsperiodeRow>(sqlx::AssertSqlSafe(sql))
        .bind(parent_id)
        .bind(offset)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await
        .map_err(Into::into)
}
