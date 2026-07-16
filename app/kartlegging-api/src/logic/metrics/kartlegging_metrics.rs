use crate::model::dao::kartlegging;
use crate::model::dao::kartlegging::KartleggingMetricsRow;
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::PgPool;
use std::sync::LazyLock;

static KARTLEGGING_GAUGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kartlegging_arbeidssoekere",
        "Kartlegging av arbeidssøkere",
        &["type"]
    )
    .expect("Failed to register kartlegging_arbeidssoekere gauge")
});

pub(crate) fn init() {}

pub(crate) async fn register_kartlegging_metrics(pg_pool: &PgPool) -> anyhow::Result<()> {
    let row = fetch_kartlegging_metrics(pg_pool).await?;
    KARTLEGGING_GAUGE
        .with_label_values(&["total"])
        .set(row.total as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["is_null"])
        .set(row.is_null as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["is_not_null"])
        .set(row.is_not_null as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_30_days"])
        .set(row.over_30_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_60_days"])
        .set(row.over_60_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_90_days"])
        .set(row.over_90_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_180_days"])
        .set(row.over_180_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_365_days"])
        .set(row.over_365_days as f64);
    Ok(())
}

async fn fetch_kartlegging_metrics(pg_pool: &PgPool) -> anyhow::Result<KartleggingMetricsRow> {
    let mut tx = pg_pool.begin().await?;
    let row = kartlegging::count_metrics(&mut tx).await?;
    tx.commit().await?;
    Ok(row)
}
