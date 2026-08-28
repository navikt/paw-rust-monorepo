use crate::kafka::hwm;
use crate::kafka::hwm::hwm_dao::HwmStatus;
use paw_rdkafka_hwm::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::Offset;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use std::time::Duration;

pub struct PausedHwm {
    pub topic: String,
    pub partition: i32,
    pub since: chrono::DateTime<chrono::Utc>,
}

pub async fn pause_and_seek(
    tx: &mut Transaction<'_, Postgres>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    version: i16,
    topic: &str,
    partition: i32,
    offset: i64,
) -> anyhow::Result<()> {
    consumer.seek(
        topic,
        partition,
        Offset::Offset(offset),
        Duration::from_secs(5),
    )?;

    let mut tpl = TopicPartitionList::new();
    tpl.add_partition(topic, partition);
    consumer.pause(&tpl)?;

    hwm::hwm_dao::update_as_paused(tx, version, topic, partition).await?;
    crate::logic::metrics::kafka_metrics::record_partition_paused(topic, partition);
    Ok(())
}

pub async fn resume_all(
    tx: &mut Transaction<'_, Postgres>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    version: i16,
) -> anyhow::Result<()> {
    let assignment = consumer.assignment()?;
    let owned_partitions: std::collections::HashSet<(String, i32)> = assignment
        .elements()
        .iter()
        .map(|tp| (tp.topic().to_string(), tp.partition()))
        .collect();

    let paused = hwm::hwm_dao::find_by_status(tx, version, HwmStatus::Paused).await?;
    let owned_paused: Vec<_> = paused
        .into_iter()
        .filter(|row| owned_partitions.contains(&(row.topic.clone(), row.partition)))
        .collect();

    let mut tpl = TopicPartitionList::new();
    for row in &owned_paused {
        tpl.add_partition(&row.topic, row.partition);
    }
    if tpl.count() == 0 {
        return Ok(());
    }
    consumer.resume(&tpl)?;
    let now = chrono::Utc::now();
    for row in &owned_paused {
        let paused_for = (now - row.status_timestamp)
            .to_std()
            .unwrap_or(Duration::ZERO);
        crate::logic::metrics::kafka_metrics::record_partition_resumed(
            &row.topic,
            row.partition,
            paused_for,
        );
    }
    Ok(())
}

pub async fn paused_partitions(
    tx: &mut Transaction<'_, Postgres>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    version: i16,
) -> Vec<PausedHwm> {
    let assignment = match consumer.assignment() {
        Ok(assignment) => assignment,
        Err(e) => {
            tracing::warn!("Kunne ikke hente consumer.assignment(): {e}");
            return Vec::new();
        }
    };
    let owned_partitions: std::collections::HashSet<(String, i32)> = assignment
        .elements()
        .iter()
        .map(|tp| (tp.topic().to_string(), tp.partition()))
        .collect();

    let paused = match hwm::hwm_dao::find_by_status(tx, version, HwmStatus::Paused).await {
        Ok(paused) => paused,
        Err(e) => {
            tracing::warn!("Kunne ikke hente pausede partisjoner fra DB: {e}");
            return Vec::new();
        }
    };

    paused
        .into_iter()
        .filter(|row| owned_partitions.contains(&(row.topic.clone(), row.partition)))
        .map(|row| PausedHwm {
            topic: row.topic,
            partition: row.partition,
            since: row.status_timestamp,
        })
        .collect()
}

pub async fn mark_resolved(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
) {
    if let Err(e) = hwm::hwm_dao::update_as_active(tx, version, topic, partition).await {
        tracing::warn!("Kunne ikke markere {topic}:{partition} som løst opp: {e}");
    }
}
