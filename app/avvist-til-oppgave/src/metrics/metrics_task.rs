use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use std::time::Duration;
use tokio::task::JoinHandle;
use crate::metrics::{avvergede_duplikate_oppgaver, avvergede_duplikater_per_dag, ekstern_oppgave_feilregistrert, gjentatte_forsok, oppgave_statuser, saksbehandlingstid};

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
    let fra_tidspunkt = metrics_cutoff();
    let mut transaction = pg_pool.begin().await?;
    oppgave_statuser::oppdater(&mut transaction).await?;
    avvergede_duplikate_oppgaver::oppdater(fra_tidspunkt, &mut transaction).await?;
    avvergede_duplikater_per_dag::oppdater(fra_tidspunkt, &mut transaction).await?;
    gjentatte_forsok::oppdater(fra_tidspunkt, &mut transaction).await?;
    saksbehandlingstid::oppdater(fra_tidspunkt, &mut transaction).await?;
    ekstern_oppgave_feilregistrert::oppdater(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}

/// Data før denne datoen er upålitelig pga. en bug
fn metrics_cutoff() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap()
}
