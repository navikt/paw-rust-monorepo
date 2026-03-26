use crate::metrics::db::hent_antall_oppgaver_per_status;
use crate::metrics::gauges::set_oppgave_status_counts;
use anyhow::Result;
use sqlx::PgPool;
use std::time::Duration;
use tokio::task::JoinHandle;

pub fn start_metrics_task(pg_pool: PgPool, interval: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(feil) = oppdater_metrikker(&pg_pool).await {
                tracing::warn!(error = %feil, "Kunne ikke oppdatere metrikker");
            }
            tokio::time::sleep(interval).await;
        }
    })
}

async fn oppdater_metrikker(pg_pool: &PgPool) -> Result<()> {
    let mut transaction = pg_pool.begin().await?;
    let oppgave_status_antall = hent_antall_oppgaver_per_status(&mut transaction).await?;
    set_oppgave_status_counts(&oppgave_status_antall);
    Ok(())
}
