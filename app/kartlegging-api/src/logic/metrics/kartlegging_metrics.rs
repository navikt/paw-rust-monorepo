use crate::model::dao::kartlegging;
use crate::model::dao::kartlegging::KartleggingMetricsRow;
use prometheus::{GaugeVec, register_gauge_vec};
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
        .with_label_values(&["is_active"])
        .set(row.is_active as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["is_not_active"])
        .set(row.is_not_active as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["is_null"])
        .set(row.is_null as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["is_not_null"])
        .set(row.is_not_null as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_0030_days"])
        .set(row.over_0030_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_0060_days"])
        .set(row.over_0060_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_0090_days"])
        .set(row.over_0090_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_0180_days"])
        .set(row.over_0180_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_0365_days"])
        .set(row.over_0365_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_0730_days"])
        .set(row.over_0730_days as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["over_1095_days"])
        .set(row.over_1095_days as f64);
    Ok(())
}

async fn fetch_kartlegging_metrics(pg_pool: &PgPool) -> anyhow::Result<KartleggingMetricsRow> {
    let mut tx = pg_pool.begin().await?;
    let row = kartlegging::count_metrics(&mut tx).await?;
    tx.commit().await?;
    Ok(row)
}
