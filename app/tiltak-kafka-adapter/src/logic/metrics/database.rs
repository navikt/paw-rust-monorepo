use crate::model::dao::arbeidssoeker;
use prometheus::{GaugeVec, register_gauge_vec};
use sqlx::PgPool;
use std::sync::LazyLock;

static DATABASE_GAUGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_tiltak_kafka_adapter_database",
        "Tiltak Kafka Adapter Database",
        &["type"]
    )
    .expect("Failed to register paw_tiltak_kafka_adapter_database gauge")
});

pub(crate) async fn register_database_metrics(pg_pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pg_pool.begin().await?;
    let count = arbeidssoeker::count(&mut tx).await?;
    tx.commit().await?;
    DATABASE_GAUGE
        .with_label_values(&["arbeidssoekere_total"])
        .set(count as f64);
    Ok(())
}
