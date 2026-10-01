use crate::model::dao::statistics::KartleggingStatisticsRow;
use crate::model::dao::statistics;
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
        .with_label_values(&["perioder_total"])
        .set(row.perioder_total as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["perioder_aktiv"])
        .set(row.perioder_aktiv as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["perioder_avsluttet"])
        .set(row.perioder_avsluttet as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_null"])
        .set(row.ledighet_null as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_ikke_null"])
        .set(row.ledighet_ikke_null as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_0030_dager"])
        .set(row.ledighet_over_0030_dager as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_0060_dager"])
        .set(row.ledighet_over_0060_dager as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_0090_dager"])
        .set(row.ledighet_over_0090_dager as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_0180_dager"])
        .set(row.ledighet_over_0180_dager as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_0365_dager"])
        .set(row.ledighet_over_0365_dager as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_0730_dager"])
        .set(row.ledighet_over_0730_dager as f64);
    KARTLEGGING_GAUGE
        .with_label_values(&["ledighet_over_1095_dager"])
        .set(row.ledighet_over_1095_dager as f64);
    Ok(())
}

async fn fetch_kartlegging_metrics(pg_pool: &PgPool) -> anyhow::Result<KartleggingStatisticsRow> {
    let mut tx = pg_pool.begin().await?;
    let row = statistics::count(&mut tx).await?;
    tx.commit().await?;
    Ok(row)
}
