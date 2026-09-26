use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeRow {
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
pub async fn select_by_arbeidssoeker_ids(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_ider: &[i64],
) -> anyhow::Result<Vec<LedighetsperiodeRow>> {
    tracing::debug!("Select ledighetsperioder by parent-ider");
    sqlx::query_as::<_, LedighetsperiodeRow>(
        r#"
        WITH
        active_perioder AS (
            SELECT DISTINCT ON (arbeidssoeker_id)
                arbeidssoeker_id,
                periode_id,
                arbeidssoeker_fra,
                arbeidssoeker_til,
                arbeidsledig_fra
            FROM kartlegginger
            WHERE arbeidssoeker_id = ANY($1) AND arbeidssoeker_til IS NULL
            ORDER BY arbeidssoeker_id, arbeidsledig_fra DESC NULLS LAST, arbeidssoeker_fra DESC
        ),
        latest_opplysninger AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, jobbsituasjon, tidspunkt
            FROM opplysninger
            WHERE periode_id IN (SELECT periode_id FROM active_perioder)
            ORDER BY periode_id, tidspunkt DESC
        ),
        latest_profileringer AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, profilert_til, tidspunkt
            FROM profileringer
            WHERE periode_id IN (SELECT periode_id FROM active_perioder)
            ORDER BY periode_id, tidspunkt DESC
        ),
        latest_egenvurderinger AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, egenvurdert_til, tidspunkt
            FROM egenvurderinger
            WHERE periode_id IN (SELECT periode_id FROM active_perioder)
            ORDER BY periode_id, tidspunkt DESC
        ),
        latest_bekreftelser AS (
            SELECT DISTINCT ON (periode_id) id, periode_id, gjelder_fra, gjelder_til,
                   har_jobbet, vil_fortsette, bekreftelsesloesning
            FROM bekreftelser
            WHERE periode_id IN (SELECT periode_id FROM active_perioder)
            ORDER BY periode_id, gjelder_til DESC
        )
        SELECT
            ap.arbeidssoeker_id,
            ap.periode_id,
            ap.arbeidssoeker_fra AT TIME ZONE 'UTC'                 AS arbeidssoeker_fra,
            ap.arbeidssoeker_til AT TIME ZONE 'UTC'                 AS arbeidssoeker_til,
            ap.arbeidsledig_fra AT TIME ZONE 'UTC'                  AS arbeidsledig_fra,
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
        FROM active_perioder ap
        LEFT JOIN perioder p                  ON p.id          = ap.periode_id
        LEFT JOIN latest_opplysninger o       ON o.periode_id  = ap.periode_id
        LEFT JOIN latest_profileringer pr     ON pr.periode_id = ap.periode_id
        LEFT JOIN latest_egenvurderinger e    ON e.periode_id  = ap.periode_id
        LEFT JOIN latest_bekreftelser b       ON b.periode_id  = ap.periode_id
        LEFT JOIN bekreftelse_paavegneav bv   ON bv.periode_id = ap.periode_id
        "#,
    )
    .bind(arbeidssoeker_ider)
    .fetch_all(&mut **tx)
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::dao::bekreftelse::BekreftelseRow;
    use crate::model::dao::egenvurdering::EgenvurderingRow;
    use crate::model::dao::kartlegging::KartleggingRow;
    use crate::model::dao::opplysninger::OpplysningerRow;
    use crate::model::dao::periode::PeriodeRow;
    use crate::model::dao::profilering::ProfileringRow;
    use crate::model::dao::{
        arbeidssoeker, bekreftelse, egenvurdering, kartlegging, opplysninger, periode, profilering,
    };
    use chrono::Duration;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use tokio::sync::OnceCell;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn select_by_arbeidssoeker_ids_korrelerer_alle_undertabeller_per_periode() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        let arbeidssoeker_id_1 = 20_001_i64;
        let arbeidssoeker_id_2 = 20_002_i64;
        let periode_id_1 = Uuid::new_v4();
        let periode_id_2 = Uuid::new_v4();
        let naa = Utc::now();

        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id_1)
            .await;
        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id_2)
            .await;

        // Periode 1 har de globalt nyeste radene i alle undertabellene.
        context
            .insert_aktiv_periode(&mut tx, periode_id_1, arbeidssoeker_id_1, naa)
            .await;
        context
            .insert_aktiv_kartlegging(&mut tx, periode_id_1, arbeidssoeker_id_1, naa, None)
            .await;
        context
            .insert_opplysninger(&mut tx, periode_id_1, "ER_PERMITTERT", naa)
            .await;
        context
            .insert_profilering(&mut tx, periode_id_1, "ANTATT_GODE_MULIGHETER", naa)
            .await;
        context
            .insert_egenvurdering(&mut tx, periode_id_1, "ANTATT_GODE_MULIGHETER", naa)
            .await;
        context
            .insert_bekreftelse(&mut tx, periode_id_1, true, false, naa)
            .await;

        // Periode 2 har eldre rader. Med de opprinnelige ukorrelerte CTE-ene ville denne
        // perioden feilaktig fått NULL i alle feltene, siden kun den globalt nyeste raden
        // i hver tabell ble plukket ut.
        let eldre = naa - Duration::hours(2);
        context
            .insert_aktiv_periode(&mut tx, periode_id_2, arbeidssoeker_id_2, eldre)
            .await;
        context
            .insert_aktiv_kartlegging(&mut tx, periode_id_2, arbeidssoeker_id_2, eldre, None)
            .await;
        context
            .insert_opplysninger(&mut tx, periode_id_2, "HAR_SAGT_OPP", eldre)
            .await;
        context
            .insert_profilering(&mut tx, periode_id_2, "OPPGITT_HINDRINGER", eldre)
            .await;
        context
            .insert_egenvurdering(&mut tx, periode_id_2, "OPPGITT_HINDRINGER", eldre)
            .await;
        context
            .insert_bekreftelse(&mut tx, periode_id_2, false, true, eldre)
            .await;

        let rows = select_by_arbeidssoeker_ids(&mut tx, &[arbeidssoeker_id_1, arbeidssoeker_id_2])
            .await
            .expect("Kunne ikke hente ledighetsperioder");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(rows.len(), 2);

        let row_1 = rows
            .iter()
            .find(|row| row.arbeidssoeker_id == arbeidssoeker_id_1)
            .expect("Fant ikke rad for arbeidssoeker 1");
        assert_eq!(row_1.periode_id, periode_id_1);
        assert_eq!(row_1.opplysninger_jobbsituasjon, vec!["ER_PERMITTERT"]);
        assert_eq!(
            row_1.profilert_til.as_deref(),
            Some("ANTATT_GODE_MULIGHETER")
        );
        assert_eq!(
            row_1.egenvurdert_til.as_deref(),
            Some("ANTATT_GODE_MULIGHETER")
        );
        assert_eq!(row_1.bekreftelse_har_jobbet, Some(true));
        assert_eq!(row_1.bekreftelse_vil_fortsette, Some(false));

        let row_2 = rows
            .iter()
            .find(|row| row.arbeidssoeker_id == arbeidssoeker_id_2)
            .expect("Fant ikke rad for arbeidssoeker 2");
        assert_eq!(row_2.periode_id, periode_id_2);
        assert_eq!(row_2.opplysninger_jobbsituasjon, vec!["HAR_SAGT_OPP"]);
        assert_eq!(row_2.profilert_til.as_deref(), Some("OPPGITT_HINDRINGER"));
        assert_eq!(row_2.egenvurdert_til.as_deref(), Some("OPPGITT_HINDRINGER"));
        assert_eq!(row_2.bekreftelse_har_jobbet, Some(false));
        assert_eq!(row_2.bekreftelse_vil_fortsette, Some(true));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn select_by_arbeidssoeker_ids_velger_nyeste_rad_per_periode() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        let arbeidssoeker_id = 20_004_i64;
        let periode_id = Uuid::new_v4();
        let naa = Utc::now();

        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id)
            .await;
        context
            .insert_aktiv_periode(&mut tx, periode_id, arbeidssoeker_id, naa)
            .await;
        context
            .insert_aktiv_kartlegging(&mut tx, periode_id, arbeidssoeker_id, naa, None)
            .await;

        context
            .insert_opplysninger(
                &mut tx,
                periode_id,
                "ER_PERMITTERT",
                naa - Duration::days(2),
            )
            .await;
        context
            .insert_opplysninger(&mut tx, periode_id, "HAR_SAGT_OPP", naa)
            .await;
        context
            .insert_bekreftelse(&mut tx, periode_id, true, true, naa - Duration::days(2))
            .await;
        context
            .insert_bekreftelse(&mut tx, periode_id, false, false, naa)
            .await;

        let rows = select_by_arbeidssoeker_ids(&mut tx, &[arbeidssoeker_id])
            .await
            .expect("Kunne ikke hente ledighetsperioder");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].opplysninger_jobbsituasjon, vec!["HAR_SAGT_OPP"]);
        assert_eq!(rows[0].bekreftelse_har_jobbet, Some(false));
        assert_eq!(rows[0].bekreftelse_vil_fortsette, Some(false));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn select_by_arbeidssoeker_ids_hopper_over_avsluttede_perioder_og_ukjente_ider() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        let arbeidssoeker_id = 20_003_i64;
        let ukjent_arbeidssoeker_id = 20_999_i64;
        let periode_id_avsluttet = Uuid::new_v4();
        let periode_id_aktiv = Uuid::new_v4();
        let naa = Utc::now();

        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id)
            .await;

        context
            .insert_avsluttet_periode(
                &mut tx,
                periode_id_avsluttet,
                arbeidssoeker_id,
                naa - Duration::days(10),
                naa - Duration::days(1),
            )
            .await;

        context
            .insert_aktiv_periode(&mut tx, periode_id_aktiv, arbeidssoeker_id, naa)
            .await;

        context
            .insert_avluttet_kartlegging(
                &mut tx,
                periode_id_avsluttet,
                arbeidssoeker_id,
                naa - Duration::days(10),
                naa - Duration::days(1),
                Some(naa - Duration::days(10)),
            )
            .await;

        context
            .insert_aktiv_kartlegging(&mut tx, periode_id_aktiv, arbeidssoeker_id, naa, None)
            .await;

        let rows =
            select_by_arbeidssoeker_ids(&mut tx, &[arbeidssoeker_id, ukjent_arbeidssoeker_id])
                .await
                .expect("Kunne ikke hente ledighetsperioder");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].periode_id, periode_id_aktiv);
        assert!(rows[0].opplysninger_id.is_none());
        assert!(rows[0].profilering_id.is_none());
        assert!(rows[0].egenvurdering_id.is_none());
        assert!(rows[0].bekreftelse_id.is_none());
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let postgres_guard = setup_postgres_container()
                .await
                .expect("Failed to start Postgres container");
            sqlx::migrate!("./migrations")
                .run(&postgres_guard.pg_pool)
                .await
                .expect("Failed to run migrations");

            TestContext {
                pg_pool: postgres_guard.pg_pool,
            }
        })
        .await
    }

    struct TestContext {
        pg_pool: PgPool,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }

        async fn insert_arbeidssoeker(&self, tx: &mut Transaction<'_, Postgres>, id: i64) {
            arbeidssoeker::insert(
                tx,
                &ArbeidssoekerRow::new(
                    id,
                    format!("2000{id}"),
                    format!("010170{id}"),
                    None,
                    None,
                    None,
                ),
            )
            .await
            .expect("Kunne ikke sette inn arbeidssøker for testoppsett");
        }

        async fn insert_aktiv_periode(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            arbeidssoeker_id: i64,
            startet: DateTime<Utc>,
        ) {
            periode::insert(
                tx,
                &PeriodeRow::new(
                    periode_id,
                    format!("010170{arbeidssoeker_id}"),
                    startet,
                    None,
                ),
            )
            .await
            .expect("Kunne ikke sette inn periode for testoppsett");
        }

        async fn insert_avsluttet_periode(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            arbeidssoeker_id: i64,
            startet: DateTime<Utc>,
            avsluttet: DateTime<Utc>,
        ) {
            periode::insert(
                tx,
                &PeriodeRow::new(
                    periode_id,
                    format!("010170{arbeidssoeker_id}"),
                    startet,
                    Some(avsluttet),
                ),
            )
            .await
            .expect("Kunne ikke sette inn periode for testoppsett");
        }

        async fn insert_aktiv_kartlegging(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            arbeidssoeker_id: i64,
            arbeidssoeker_fra: DateTime<Utc>,
            arbeidsledig_fra: Option<DateTime<Utc>>,
        ) {
            kartlegging::insert(
                tx,
                &KartleggingRow::new(
                    periode_id,
                    arbeidssoeker_id,
                    arbeidssoeker_fra,
                    None,
                    arbeidsledig_fra,
                ),
            )
            .await
            .expect("Kunne ikke sette inn kartlegging for testoppsett");
        }

        async fn insert_avluttet_kartlegging(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            arbeidssoeker_id: i64,
            arbeidssoeker_fra: DateTime<Utc>,
            arbeidssoeker_til: DateTime<Utc>,
            arbeidsledig_fra: Option<DateTime<Utc>>,
        ) {
            kartlegging::insert(
                tx,
                &KartleggingRow::new(
                    periode_id,
                    arbeidssoeker_id,
                    arbeidssoeker_fra,
                    Some(arbeidssoeker_til),
                    arbeidsledig_fra,
                ),
            )
            .await
            .expect("Kunne ikke sette inn kartlegging for testoppsett");
        }

        async fn insert_opplysninger(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            jobbsituasjon: &str,
            tidspunkt: DateTime<Utc>,
        ) {
            opplysninger::insert(
                tx,
                &OpplysningerRow::new(
                    Uuid::new_v4(),
                    periode_id,
                    vec![jobbsituasjon.to_string()],
                    tidspunkt,
                ),
            )
            .await
            .expect("Kunne ikke sette inn opplysninger for testoppsett");
        }

        async fn insert_profilering(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            profilert_til: &str,
            tidspunkt: DateTime<Utc>,
        ) {
            profilering::insert(
                tx,
                &ProfileringRow::new(
                    Uuid::new_v4(),
                    periode_id,
                    Uuid::new_v4(),
                    profilert_til.to_string(),
                    tidspunkt,
                ),
            )
            .await
            .expect("Kunne ikke sette inn profilering for testoppsett");
        }

        async fn insert_egenvurdering(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            egenvurdert_til: &str,
            tidspunkt: DateTime<Utc>,
        ) {
            egenvurdering::insert(
                tx,
                &EgenvurderingRow::new(
                    Uuid::new_v4(),
                    periode_id,
                    Uuid::new_v4(),
                    "ANTATT_GODE_MULIGHETER".to_string(),
                    egenvurdert_til.to_string(),
                    tidspunkt,
                ),
            )
            .await
            .expect("Kunne ikke sette inn egenvurdering for testoppsett");
        }

        async fn insert_bekreftelse(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            har_jobbet: bool,
            vil_fortsette: bool,
            gjelder_til: DateTime<Utc>,
        ) {
            bekreftelse::insert(
                tx,
                &BekreftelseRow::new(
                    Uuid::new_v4(),
                    periode_id,
                    gjelder_til - Duration::days(14),
                    gjelder_til,
                    har_jobbet,
                    vil_fortsette,
                    "ARBEIDSSOEKERREGISTERET".to_string(),
                    gjelder_til,
                ),
            )
            .await
            .expect("Kunne ikke sette inn bekreftelse for testoppsett");
        }
    }
}
