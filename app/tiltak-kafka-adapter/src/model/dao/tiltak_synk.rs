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

impl TiltakSynkRow {
    pub fn new(
        periode_id: Uuid,
        synk_id: Uuid,
        status: SynkStatus,
        status_endret: DateTime<Utc>,
        sist_kjoert: DateTime<Utc>,
        antall_feil: i32,
    ) -> Self {
        Self {
            periode_id,
            synk_id,
            status,
            status_endret,
            sist_kjoert,
            antall_feil,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct BatchTiltakSynkRow {
    pub periode_id: Uuid,
    pub identitetsnummer: String,
}

#[tracing::instrument(skip_all)]
pub async fn count_by_status_venter<'a>(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<i64> {
    let finnes = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM tiltak_synk WHERE status = 'VENTER'
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(finnes)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_periode_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &'a Uuid,
) -> anyhow::Result<Option<TiltakSynkRow>> {
    let optional_row = sqlx::query_as::<_, TiltakSynkRow>(
        r#"
        SELECT
            periode_id,
            synk_id,
            status,
            status_endret AT TIME ZONE 'UTC' AS status_endret,
            sist_kjoert   AT TIME ZONE 'UTC' AS sist_kjoert,
            antall_feil
        FROM tiltak_synk WHERE periode_id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(optional_row)
}

#[tracing::instrument(skip_all)]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a TiltakSynkRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
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
pub async fn select_batch<'a>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dao::arbeidssoeker::{self, ArbeidssoekerRow};
    use chrono::Duration;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use std::collections::HashSet;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claim_tar_bare_venter_eldste_foerst_og_respekterer_grense() {
        let pool = init().await;

        let ventende = opprett_venter(&pool, 5).await;
        let ferdig = opprett(&pool, SynkStatus::Ferdig, 1000).await;
        let feilet = opprett(&pool, SynkStatus::Feilet, 1000).await;
        let synk_id = Uuid::new_v4();

        let barch_rows = hent_batch(&pool, &synk_id, 3).await;
        let batch_rows_ids: HashSet<Uuid> = barch_rows.iter().map(|r| r.periode_id).collect();

        let first_3_ventende: HashSet<Uuid> = ventende[..3].iter().copied().collect();
        assert_eq!(batch_rows_ids, first_3_ventende);
        assert!(
            barch_rows
                .iter()
                .all(|r| r.identitetsnummer == "01017012345")
        );
        for periode_id in &first_3_ventende {
            let row = hent_for_periode_id(&pool, periode_id).await;
            assert_eq!(row.synk_id, synk_id);
            assert_eq!(row.status, SynkStatus::Aktiv);
        }
        let row_1 = hent_for_periode_id(&pool, &ventende[3]).await;
        assert_eq!(row_1.status, SynkStatus::Venter);
        let row_2 = hent_for_periode_id(&pool, &ferdig).await;
        assert_eq!(row_2.status, SynkStatus::Ferdig);
        let row_3 = hent_for_periode_id(&pool, &feilet).await;
        assert_eq!(row_3.status, SynkStatus::Feilet);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn claim_venter_paa_ucommittet_claim_og_filtrerer_bort_radene() {
        let pool = init().await;

        let ventende = opprett_venter(&pool, 4).await;
        let synk_id_1 = Uuid::new_v4();
        let synk_id_2 = Uuid::new_v4();

        let mut tx_a = pool.begin().await.expect("Kunne ikke starte tx");
        let barch_rows_1 = select_batch(&mut tx_a, &synk_id_1, 2).await.unwrap();
        let batch_rows_1_ids: HashSet<Uuid> = barch_rows_1.iter().map(|r| r.periode_id).collect();

        let first_2_ids: HashSet<Uuid> = ventende[..2].iter().copied().collect();
        assert_eq!(batch_rows_1_ids, first_2_ids);

        let cloned_pool = pool.clone();
        let handle = tokio::spawn(async move { hent_batch(&cloned_pool, &synk_id_2, 4).await });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(!handle.is_finished(), "B skal vente på radlåsen til A");

        tx_a.commit().await.expect("Kunne ikke comitte tx");

        let batch_rows_2 = handle.await.unwrap();
        let batch_rows_2_ids: HashSet<Uuid> = batch_rows_2.iter().map(|r| r.periode_id).collect();

        let last_2_ids: HashSet<Uuid> = ventende[2..].iter().copied().collect();
        assert_eq!(batch_rows_2_ids, last_2_ids);
        for periode_id in &first_2_ids {
            let row = hent_for_periode_id(&pool, periode_id).await;
            assert_eq!(row.synk_id, synk_id_1);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn samtidige_claims_gir_disjunkte_sett() {
        let pool = init().await;

        let ventende: HashSet<Uuid> = opprett_venter(&pool, 200).await.into_iter().collect();

        let mut handles = Vec::new();
        for _ in 0..8 {
            let cloned_pool = pool.clone();
            handles.push(tokio::spawn(async move {
                let synk_id = Uuid::new_v4();
                let mut claimet = Vec::new();
                for _ in 0..10 {
                    let rows = hent_batch(&cloned_pool, &synk_id, 10).await;
                    let ids: Vec<Uuid> = rows.iter().map(|r| r.periode_id).collect();
                    claimet.extend(ids);
                }
                claimet
            }));
        }

        let mut union = HashSet::new();
        let mut totalt = 0;
        for handle in handles {
            let sett = handle.await.unwrap();
            totalt += sett.len();
            union.extend(sett);
        }

        assert_eq!(totalt, union.len(), "samme rad claimet av flere");
        assert!(union.is_subset(&ventende));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn finnes_ventende_svarer_paa_om_venter_gjenstaar() {
        let pool = init().await;

        assert_eq!(tell_venter(&pool).await, 0);

        opprett(&pool, SynkStatus::Ferdig, 0).await;
        assert_eq!(tell_venter(&pool).await, 0);

        opprett(&pool, SynkStatus::Venter, 0).await;
        assert_eq!(tell_venter(&pool).await, 1);
    }

    async fn init() -> PgPool {
        let guard = setup_postgres_container()
            .await
            .expect("Failed to start Postgres container");
        sqlx::migrate!("./migrations")
            .run(&guard.pg_pool)
            .await
            .expect("Failed to run migrations");

        guard.pg_pool
    }

    async fn tell_venter<'a>(pool: &'a PgPool) -> i64 {
        let mut tx = pool.begin().await.expect("Kunne ikke starte tx");
        let count = count_by_status_venter(&mut tx).await.unwrap();
        tx.commit().await.expect("Kunne ikke comitte tx");
        count
    }

    async fn hent_batch<'a>(
        pool: &'a PgPool,
        synk_id: &'a Uuid,
        batch_stoerrelse: i64,
    ) -> Vec<BatchTiltakSynkRow> {
        let mut tx = pool.begin().await.expect("Kunne ikke starte tx");
        let row = select_batch(&mut tx, &synk_id, batch_stoerrelse)
            .await
            .unwrap();
        tx.commit().await.expect("Kunne ikke comitte tx");
        row
    }

    async fn hent_for_periode_id<'a>(pool: &'a PgPool, periode_id: &'a Uuid) -> TiltakSynkRow {
        let mut tx = pool.begin().await.expect("Kunne ikke starte tx");
        let row = select_by_periode_id(&mut tx, &periode_id).await.unwrap();
        tx.commit().await.expect("Kunne ikke comitte tx");
        assert!(row.is_some());
        row.unwrap()
    }

    async fn opprett<'a>(pool: &'a PgPool, status: SynkStatus, alder_minutter: i64) -> Uuid {
        let periode_id = Uuid::new_v4();
        let naa = Utc::now();
        let row = TiltakSynkRow::new(
            periode_id,
            Uuid::new_v4(),
            status,
            naa,
            naa - Duration::minutes(alder_minutter),
            0,
        );

        let mut tx = pool.begin().await.expect("Kunne ikke starte tx");

        arbeidssoeker::insert(
            &mut tx,
            &ArbeidssoekerRow {
                periode_id,
                identitetsnummer: "01017012345".to_string(),
                periode_startet: naa,
            },
        )
        .await
        .unwrap();

        insert(&mut tx, &row).await.unwrap();
        tx.commit().await.expect("Kunne ikke comitte tx");
        periode_id
    }

    async fn opprett_venter(pool: &PgPool, antall: i64) -> Vec<Uuid> {
        let mut ider = Vec::new();
        for i in 0..antall {
            ider.push(opprett(&pool, SynkStatus::Venter, antall - i).await);
        }
        ider
    }
}
