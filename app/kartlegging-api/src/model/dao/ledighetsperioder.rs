use crate::model::sort::SortOrder;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeRow {
    pub id: i64,
    pub parent_id: i64,
    pub ledig_siden: Option<DateTime<Utc>>,
    pub periode_id: Uuid,
    pub periode_startet: DateTime<Utc>,
    pub periode_avsluttet: Option<DateTime<Utc>>,
    pub opplysninger_id: Option<Uuid>,
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
    pub inserted_timestamp: DateTime<Utc>,
    pub updated_timestamp: Option<DateTime<Utc>>,
}

impl LedighetsperiodeRow {
    pub fn new(
        parent_id: i64,
        periode_id: Uuid,
        periode_startet: DateTime<Utc>,
        periode_avsluttet: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: -1,
            ledig_siden: None,
            parent_id,
            periode_id,
            periode_startet,
            periode_avsluttet,
            opplysninger_id: None,
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
pub async fn count_by_tilknyttet_kontor(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &NaiveDate,
) -> anyhow::Result<i64> {
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) AS count
        FROM arbeidssoekere l LEFT JOIN tilknyttet_kontor tk on l.id = tk.parent_id
        WHERE tk.kontor_id = $1 AND tk.kontor_type = ANY($2) AND l.ledig_siden NOTNULL AND l.ledig_siden > $3
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
pub async fn select_by_parent_id(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: i64,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<LedighetsperiodeRow>> {
    match sort_order {
        SortOrder::Ascending => select_by_parent_id_asc(tx, parent_id, offset, limit).await,
        SortOrder::Descending => select_by_parent_id_desc(tx, parent_id, offset, limit).await,
    }
}

#[tracing::instrument(skip(tx))]
async fn select_by_parent_id_asc(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: i64,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<LedighetsperiodeRow>> {
    let rows = sqlx::query_as::<_, LedighetsperiodeRow>(
        r#"
        SELECT
            l.id,
            l.parent_id,
            l.ledig_siden AT TIME ZONE 'UTC' AS ledig_siden,
            l.periode_id,
            l.periode_startet AT TIME ZONE 'UTC' AS periode_startet,
            l.periode_avsluttet AT TIME ZONE 'UTC' AS periode_avsluttet,
            l.opplysninger_id,
            l.opplysninger_tidspunkt AT TIME ZONE 'UTC' AS opplysninger_tidspunkt,
            l.profilering_id,
            l.profilert_til,
            l.profilering_tidspunkt AT TIME ZONE 'UTC' AS profilering_tidspunkt,
            l.egenvurdering_id,
            l.egenvurdert_til,
            l.egenvurdering_tidspunkt AT TIME ZONE 'UTC' AS egenvurdering_tidspunkt,
            l.bekreftelse_id,
            l.bekreftelse_gjelder_fra AT TIME ZONE 'UTC' AS bekreftelse_gjelder_fra,
            l.bekreftelse_gjelder_til AT TIME ZONE 'UTC' AS bekreftelse_gjelder_til,
            l.bekreftelse_har_jobbet,
            l.bekreftelse_vil_fortsette,
            l.bekreftelsesloesning,
            l.bekreftelse_paa_vegne_av,
            l.inserted_timestamp AT TIME ZONE 'UTC' AS inserted_timestamp,
            l.updated_timestamp AT TIME ZONE 'UTC' AS updated_timestamp
        FROM ledighetsperioder l
        WHERE l.parent_id = $1
        ORDER BY l.periode_startet
        OFFSET $2
        LIMIT $3
        "#,
    )
    .bind(parent_id)
    .bind(offset)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx))]
async fn select_by_parent_id_desc(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: i64,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<LedighetsperiodeRow>> {
    let rows = sqlx::query_as::<_, LedighetsperiodeRow>(
        r#"
        SELECT
            l.id,
            l.parent_id,
            l.ledig_siden AT TIME ZONE 'UTC' AS ledig_siden,
            l.periode_id,
            l.periode_startet AT TIME ZONE 'UTC' AS periode_startet,
            l.periode_avsluttet AT TIME ZONE 'UTC' AS periode_avsluttet,
            l.opplysninger_id,
            l.opplysninger_tidspunkt AT TIME ZONE 'UTC' AS opplysninger_tidspunkt,
            l.profilering_id,
            l.profilert_til,
            l.profilering_tidspunkt AT TIME ZONE 'UTC' AS profilering_tidspunkt,
            l.egenvurdering_id,
            l.egenvurdert_til,
            l.egenvurdering_tidspunkt AT TIME ZONE 'UTC' AS egenvurdering_tidspunkt,
            l.bekreftelse_id,
            l.bekreftelse_gjelder_fra AT TIME ZONE 'UTC' AS bekreftelse_gjelder_fra,
            l.bekreftelse_gjelder_til AT TIME ZONE 'UTC' AS bekreftelse_gjelder_til,
            l.bekreftelse_har_jobbet,
            l.bekreftelse_vil_fortsette,
            l.bekreftelsesloesning,
            l.bekreftelse_paa_vegne_av,
            l.inserted_timestamp AT TIME ZONE 'UTC' AS inserted_timestamp,
            l.updated_timestamp AT TIME ZONE 'UTC' AS updated_timestamp
        FROM ledighetsperioder l
        WHERE l.parent_id = $1
        ORDER BY l.periode_startet DESC
        OFFSET $2
        LIMIT $3
        "#,
    )
    .bind(parent_id)
    .bind(offset)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx))]
pub async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    row: &LedighetsperiodeRow,
) -> anyhow::Result<i64> {
    let id = sqlx::query_scalar(
        r#"
        INSERT INTO ledighetsperioder (
            parent_id,
            ledig_siden,
            periode_id,
            periode_startet,
            periode_avsluttet,
            opplysninger_id,
            opplysninger_tidspunkt,
            profilering_id,
            profilert_til,
            profilering_tidspunkt,
            egenvurdering_id,
            egenvurdert_til,
            egenvurdering_tidspunkt,
            bekreftelse_id,
            bekreftelse_gjelder_fra,
            bekreftelse_gjelder_til,
            bekreftelse_har_jobbet,
            bekreftelse_vil_fortsette,
            bekreftelsesloesning,
            bekreftelse_paa_vegne_av,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
        RETURNING id
        "#,
    )
            .bind(&row.periode_id)
            .bind(row.ledig_siden)
            .bind(row.periode_id)
            .bind(row.periode_startet)
            .bind(row.periode_avsluttet)
            .bind(row.opplysninger_id)
            .bind(row.opplysninger_tidspunkt)
            .bind(row.profilering_id)
            .bind(&row.profilert_til)
            .bind(row.profilering_tidspunkt)
            .bind(row.egenvurdering_id)
            .bind(&row.egenvurdert_til)
            .bind(row.egenvurdering_tidspunkt)
            .bind(row.bekreftelse_id)
            .bind(row.bekreftelse_gjelder_fra)
            .bind(row.bekreftelse_gjelder_til)
            .bind(row.bekreftelse_har_jobbet)
            .bind(row.bekreftelse_vil_fortsette)
            .bind(&row.bekreftelsesloesning)
            .bind(&row.bekreftelse_paa_vegne_av)
            .bind(Utc::now())
            .fetch_one(&mut **tx)
            .await?;
    Ok(id)
}

#[tracing::instrument(skip(tx))]
pub async fn update(
    tx: &mut Transaction<'_, Postgres>,
    row: &LedighetsperiodeRow,
) -> anyhow::Result<i64> {
    let id = sqlx::query_scalar(
        r#"
        UPDATE ledighetsperioder SET (
            ledig_siden,
            periode_id,
            periode_startet,
            periode_avsluttet,
            opplysninger_id,
            opplysninger_tidspunkt,
            profilering_id,
            profilert_til,
            profilering_tidspunkt,
            egenvurdering_id,
            egenvurdert_til,
            egenvurdering_tidspunkt,
            bekreftelse_id,
            bekreftelse_gjelder_fra,
            bekreftelse_gjelder_til,
            bekreftelse_har_jobbet,
            bekreftelse_vil_fortsette,
            bekreftelsesloesning,
            bekreftelse_paa_vegne_av,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21) WHERE id = $1
        RETURNING id
        "#,
    )
    .bind(&row.ledig_siden)
    .bind(&row.periode_id)
    .bind(&row.periode_startet)
    .bind(&row.periode_avsluttet)
    .bind(&row.opplysninger_id)
    .bind(&row.opplysninger_tidspunkt)
    .bind(&row.profilering_id)
    .bind(&row.profilert_til)
    .bind(&row.profilering_tidspunkt)
    .bind(&row.egenvurdering_id)
    .bind(&row.egenvurdert_til)
    .bind(&row.egenvurdering_tidspunkt)
    .bind(&row.bekreftelse_id)
    .bind(&row.bekreftelse_gjelder_fra)
    .bind(&row.bekreftelse_gjelder_til)
    .bind(&row.bekreftelse_har_jobbet)
    .bind(&row.bekreftelse_vil_fortsette)
    .bind(&row.bekreftelse_paa_vegne_av)
    .bind(Utc::now())
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}
