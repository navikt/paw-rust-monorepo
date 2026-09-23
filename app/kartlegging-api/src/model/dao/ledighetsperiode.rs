use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeRow {
    #[allow(unused)]
    pub arbeidssoeker_id: i64,
    pub periode_id: Uuid,
    #[allow(unused)]
    pub arbeidssoeker_fra: DateTime<Utc>,
    #[allow(unused)]
    pub arbeidssoeker_til: Option<DateTime<Utc>>,
    pub arbeidsledig_fra: Option<DateTime<Utc>>,
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

#[tracing::instrument(skip_all)]
pub async fn select_by_arbeidssoeker_id(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: i64,
) -> anyhow::Result<Vec<LedighetsperiodeRow>> {
    tracing::debug!("Select ledighetsperioder by parent-id");
    sqlx::query_as::<_, LedighetsperiodeRow>(
        r#"
        WITH latest_opplysninger AS (
            SELECT id, periode_id, jobbsituasjon, tidspunkt
            FROM opplysninger
            ORDER BY tidspunkt DESC
            LIMIT 1
        ),
        latest_profileringer AS (
            SELECT id, periode_id, profilert_til, tidspunkt
            FROM profileringer
            ORDER BY tidspunkt DESC
            LIMIT 1
        ),
        latest_egenvurderinger AS (
            SELECT id, periode_id, egenvurdert_til, tidspunkt
            FROM egenvurderinger
            ORDER BY tidspunkt DESC
            LIMIT 1
        ),
        latest_bekreftelser AS (
            SELECT id, periode_id, gjelder_fra, gjelder_til,
                   har_jobbet, vil_fortsette, bekreftelsesloesning
            FROM bekreftelser
            ORDER BY gjelder_til DESC
            LIMIT 1
        )
        SELECT
            k.arbeidssoeker_id,
            k.periode_id,
            k.arbeidssoeker_fra AT TIME ZONE 'UTC'                  AS arbeidssoeker_fra,
            k.arbeidssoeker_til AT TIME ZONE 'UTC'                  AS arbeidssoeker_til,
            k.arbeidsledig_fra AT TIME ZONE 'UTC'                   AS arbeidsledig_fra,
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
        WHERE k.arbeidssoeker_id = $1 AND k.arbeidssoeker_til IS NOT NULL
        ORDER BY k.arbeidsledig_fra DESC NULLS LAST, k.arbeidssoeker_fra DESC
        LIMIT 1
        "#,
    )
    .bind(arbeidssoeker_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(Into::into)
}
