use crate::metrics::{avvergede_duplikate_oppgaver, avvergede_duplikater_per_dag, oppgave_statuser};
use anyhow::Result;
use sqlx::PgPool;
use std::time::Duration;
use tokio::task::JoinHandle;

const METRIKK_TASK_INTERVALL: Duration = Duration::from_secs(60);

pub fn start_metrics_task(pg_pool: PgPool) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(feil) = oppdater_metrikker(&pg_pool).await {
                tracing::warn!(error = %feil, "Kunne ikke oppdatere metrikker");
            }
            tokio::time::sleep(METRIKK_TASK_INTERVALL).await;
        }
    })
}

async fn oppdater_metrikker(pg_pool: &PgPool) -> Result<()> {
    let mut transaction = pg_pool.begin().await?;
    oppgave_statuser::oppdater(&mut transaction).await?;
    avvergede_duplikate_oppgaver::oppdater(&mut transaction).await?;
    avvergede_duplikater_per_dag::oppdater(&mut transaction).await?;
    Ok(())
}
