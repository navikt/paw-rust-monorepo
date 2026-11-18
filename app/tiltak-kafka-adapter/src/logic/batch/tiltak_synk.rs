use crate::model::dao::tiltak_synk::{self, BatchTiltakSynkRow};
use sqlx::PgPool;
use std::ops::ControlFlow;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum BatchSlutt {
    IngenVentende,
    ForMangeTommeClaims,
    Stoppet,
}

#[tracing::instrument(skip_all)]
pub async fn for_hver_batch<'a>(
    pool: &'a PgPool,
    synk_id: &'a Uuid,
    batch_stoerrelse: i64,
    maks_tomme_claims: u32,
    mut behandle: impl AsyncFnMut(Vec<BatchTiltakSynkRow>) -> ControlFlow<()>,
) -> anyhow::Result<BatchSlutt> {
    let mut empty_rows = 0;
    loop {
        let mut batch_tx = pool.begin().await?;
        let rows = tiltak_synk::select_batch(&mut batch_tx, synk_id, batch_stoerrelse).await?;
        batch_tx.commit().await?;

        if rows.is_empty() {
            let mut count_tx = pool.begin().await?;
            let count = tiltak_synk::count_by_status_venter(&mut count_tx).await?;
            count_tx.commit().await?;

            if count == 0 {
                return Ok(BatchSlutt::IngenVentende);
            }
            empty_rows += 1;
            if empty_rows >= maks_tomme_claims {
                tracing::info!(empty_rows, "Gir opp claim, neste kjøring tar resten");
                return Ok(BatchSlutt::ForMangeTommeClaims);
            }
            continue;
        }
        empty_rows = 0;
        if behandle(rows).await.is_break() {
            return Ok(BatchSlutt::Stoppet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dao::arbeidssoeker::{self, ArbeidssoekerRow};
    use crate::model::dao::tiltak_synk::{TiltakSynkRow, count_by_status_venter};
    use crate::model::status::SynkStatus;
    use chrono::Utc;
    use postgres_testcontainer::postgres::setup_postgres_container;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn behandler_alle_batcher_til_ingen_venter() {
        let pool = init().await;
        opprett_venter(&pool, 25).await;
        let mut batcher = Vec::new();

        let slutt = for_hver_batch(&pool, &Uuid::new_v4(), 10, 3, async |rader| {
            batcher.push(rader.len());
            ControlFlow::Continue(())
        })
        .await
        .unwrap();

        assert_eq!(slutt, BatchSlutt::IngenVentende);
        assert_eq!(batcher, vec![10, 10, 5]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gir_opp_etter_maks_tomme_claims_naar_venter_gjenstaar() {
        let pool = init().await;
        opprett_venter(&pool, 1).await;

        let slutt = for_hver_batch(&pool, &Uuid::new_v4(), 0, 3, async |_| {
            panic!("skal ikke behandle noe")
        })
        .await
        .unwrap();

        assert_eq!(slutt, BatchSlutt::ForMangeTommeClaims);
        assert_eq!(tell_venter(&pool).await, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stopper_naar_behandling_ber_om_det() {
        let pool = init().await;
        opprett_venter(&pool, 25).await;
        let mut kall = 0;

        let slutt = for_hver_batch(&pool, &Uuid::new_v4(), 10, 3, async |_| {
            kall += 1;
            ControlFlow::Break(())
        })
        .await
        .unwrap();

        assert_eq!(slutt, BatchSlutt::Stoppet);
        assert_eq!(kall, 1);
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

    async fn opprett_venter(pool: &PgPool, antall: usize) {
        let now = Utc::now();
        let mut tx = pool.begin().await.expect("Kunne ikke starte tx");
        for _ in 0..antall {
            let periode_id = Uuid::new_v4();
            let row =
                TiltakSynkRow::new(periode_id, Uuid::new_v4(), SynkStatus::Venter, now, now, 0);

            arbeidssoeker::insert(
                &mut tx,
                &ArbeidssoekerRow {
                    periode_id,
                    identitetsnummer: "01017012345".to_string(),
                    periode_startet: now,
                },
            )
            .await
            .unwrap();

            tiltak_synk::insert(&mut tx, &row).await.unwrap();
        }
        tx.commit().await.expect("Kunne ikke comitte tx");
    }
}
