use crate::metrics::{
    avvist_under_18, ekstern_oppgave_feilregistrert, oppgave_statuser, saksbehandlingstid,
};
use anyhow::Result;
use avvist_under_18::{
    avvergede_duplikate_oppgaver, avvergede_duplikater_per_dag, gjentatte_forsok,
};
use sqlx::PgPool;
use std::time::Duration;
use tokio::task::JoinHandle;

const METRIKK_TASK_INTERVALL: Duration = Duration::from_secs(300);

pub fn spawn_metrics_task(pg_pool: PgPool) -> JoinHandle<()> {
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
    let avvist_under_18_cutoff = avvist_under_18::cutoff_date();
    let mut transaction = pg_pool.begin().await?;
    oppgave_statuser::oppdater(&mut transaction).await?;
    avvergede_duplikate_oppgaver::oppdater(avvist_under_18_cutoff, &mut transaction).await?;
    avvergede_duplikater_per_dag::oppdater(avvist_under_18_cutoff, &mut transaction).await?;
    gjentatte_forsok::oppdater(avvist_under_18_cutoff, &mut transaction).await?;
    saksbehandlingstid::oppdater(avvist_under_18_cutoff, &mut transaction).await?;
    ekstern_oppgave_feilregistrert::oppdater(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}
