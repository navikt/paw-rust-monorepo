use crate::config::AppConfig;
use crate::logic::metrics::kartlegging_metrics::register_kartlegging_metrics;
use sqlx::PgPool;
use tokio::task::JoinHandle;

#[tracing::instrument(skip(app_config, pg_pool))]
pub fn metrics_task(app_config: AppConfig, pg_pool: PgPool) -> JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move {
        loop {
            tracing::debug!("Kjører task for oppdatering av metrikker");
            if let Err(e) = register_kartlegging_metrics(&pg_pool).await {
                tracing::warn!(error = %e, "Kunne ikke oppdatere metrikker");
            }
            tokio::time::sleep(app_config.metrics_task_interval).await;
        }
    })
}
