use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeV2Row {
    pub arbeidssoeker_id: i64,
    pub periode_id: Uuid,
    pub arbeidssoeker_fra: DateTime<Utc>,
    pub arbeidsledig_fra: Option<DateTime<Utc>>,
    #[allow(unused)]
    pub arbeidssoeker_til: Option<DateTime<Utc>>,
    pub egenvurdert_til: Option<String>,
    pub bekreftelse_har_jobbet: Option<bool>,
    pub bekreftelse_vil_fortsette: Option<bool>,
    pub bekreftelse_ansvar: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn select_by_arbeidssoeker_ids(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_ider: &[i64],
) -> anyhow::Result<Vec<LedighetsperiodeV2Row>> {
    tracing::debug!("Select ledighetsperioder by parent-ider");
    let rows = sqlx::query_as::<_, LedighetsperiodeV2Row>(
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
        latest_egenvurderinger AS (
            SELECT DISTINCT ON (periode_id) periode_id, egenvurdert_til
            FROM egenvurderinger
            WHERE periode_id IN (SELECT periode_id FROM active_perioder)
            ORDER BY periode_id, tidspunkt DESC
        ),
        latest_bekreftelser AS (
            SELECT DISTINCT ON (periode_id) periode_id, har_jobbet, vil_fortsette
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
            e.egenvurdert_til,
            b.har_jobbet                                            AS bekreftelse_har_jobbet,
            b.vil_fortsette                                         AS bekreftelse_vil_fortsette,
            COALESCE(bv.bekreftelsesloesninger, ARRAY[]::varchar[]) AS bekreftelse_ansvar
        FROM active_perioder ap
        LEFT JOIN latest_egenvurderinger e    ON e.periode_id  = ap.periode_id
        LEFT JOIN latest_bekreftelser b       ON b.periode_id  = ap.periode_id
        LEFT JOIN bekreftelse_paavegneav bv   ON bv.periode_id = ap.periode_id
        "#,
    )
    .bind(arbeidssoeker_ider)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::dao::bekreftelse::BekreftelseRow;
    use crate::model::dao::egenvurdering::EgenvurderingRow;
    use crate::model::dao::kartlegging::KartleggingRow;
    use crate::model::dao::{arbeidssoeker, bekreftelse, egenvurdering, kartlegging};
    use chrono::Duration;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use tokio::sync::OnceCell;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn select_by_arbeidssoeker_ids_korrelerer_egenvurdering_og_bekreftelse_per_periode() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        let arbeidssoeker_id_1 = 10_001_i64;
        let arbeidssoeker_id_2 = 10_002_i64;
        let periode_id_1 = Uuid::new_v4();
        let periode_id_2 = Uuid::new_v4();
        let naa = Utc::now();

        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id_1)
            .await;
        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id_2)
            .await;

        // Periode 1: nyeste egenvurdering/bekreftelse i hele tabellen.
        context
            .insert_aktiv_kartlegging(&mut tx, periode_id_1, arbeidssoeker_id_1, naa, None)
            .await;
        context
            .insert_egenvurdering(&mut tx, periode_id_1, "ANTATT_GODE_MULIGHETER", naa)
            .await;
        context
            .insert_bekreftelse(&mut tx, periode_id_1, true, false, naa)
            .await;

        // Periode 2: eldre egenvurdering/bekreftelse enn periode 1. Med den opprinnelige
        // (ukorrelerte) spørringen ville denne perioden feilaktig fått NULL, siden bare
        // den globalt nyeste raden i hele tabellen ble plukket ut.
        context
            .insert_aktiv_kartlegging(
                &mut tx,
                periode_id_2,
                arbeidssoeker_id_2,
                naa - Duration::hours(2),
                None,
            )
            .await;
        context
            .insert_egenvurdering(
                &mut tx,
                periode_id_2,
                "OPPGITT_HINDRINGER",
                naa - Duration::hours(1),
            )
            .await;
        context
            .insert_bekreftelse(&mut tx, periode_id_2, false, true, naa - Duration::hours(1))
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
        assert_eq!(row_2.egenvurdert_til.as_deref(), Some("OPPGITT_HINDRINGER"));
        assert_eq!(row_2.bekreftelse_har_jobbet, Some(false));
        assert_eq!(row_2.bekreftelse_vil_fortsette, Some(true));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn select_by_arbeidssoeker_ids_hopper_over_avsluttede_perioder_og_ukjente_ider() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        let arbeidssoeker_id = 10_003_i64;
        let ukjent_arbeidssoeker_id = 10_999_i64;
        let periode_id_avsluttet = Uuid::new_v4();
        let periode_id_aktiv = Uuid::new_v4();
        let naa = Utc::now();

        context
            .insert_arbeidssoeker(&mut tx, arbeidssoeker_id)
            .await;

        context
            .insert_avluttet_kartlegging(
                &mut tx,
                periode_id_avsluttet,
                arbeidssoeker_id,
                naa - Duration::days(10),
                naa - Duration::days(1),
                None,
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
        assert!(rows[0].egenvurdert_til.is_none());
        assert!(rows[0].bekreftelse_har_jobbet.is_none());
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
            self.insert_kartlegging(
                tx,
                periode_id,
                arbeidssoeker_id,
                arbeidssoeker_fra,
                Some(arbeidssoeker_til),
                arbeidsledig_fra,
            )
            .await
        }

        async fn insert_kartlegging(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            periode_id: Uuid,
            arbeidssoeker_id: i64,
            arbeidssoeker_fra: DateTime<Utc>,
            arbeidssoeker_til: Option<DateTime<Utc>>,
            arbeidsledig_fra: Option<DateTime<Utc>>,
        ) {
            kartlegging::insert(
                tx,
                &KartleggingRow::new(
                    periode_id,
                    arbeidssoeker_id,
                    arbeidssoeker_fra,
                    arbeidssoeker_til,
                    arbeidsledig_fra,
                ),
            )
            .await
            .expect("Kunne ikke sette inn kartlegging for testoppsett");
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
