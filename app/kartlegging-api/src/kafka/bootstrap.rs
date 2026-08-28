use anyhow::Result;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::hwm::DEFAULT_HWM_OFFSET;
use paw_rdkafka_hwm::hwm_functions::{get_hwm, insert_hwm};
use rdkafka::consumer::{BaseConsumer, Consumer};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

const LOOKBACK: i64 = 100;

const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(10);

// TODO: Fjern før prodsetting!!!
pub async fn bootstrap_missing_hwms(
    pg_pool: &PgPool,
    kafka_config: Arc<KafkaConfig>,
    topics: &[&str],
) -> Result<()> {
    let version = *kafka_config.hwm_version;
    let mut client_config = kafka_config.rdkafka_client_config()?;
    let group_id = client_config
        .get("group.id")
        .unwrap_or_default()
        .to_string();
    client_config.set("group.id", format!("{group_id}-hwm-bootstrap"));
    let consumer: BaseConsumer = client_config.create()?;

    let mut tx = pg_pool.begin().await?;
    for topic in topics {
        let metadata = consumer.fetch_metadata(Some(topic), BOOTSTRAP_TIMEOUT)?;
        for topic_metadata in metadata.topics() {
            for partition_metadata in topic_metadata.partitions() {
                let partition_i32 = partition_metadata.id();
                let partition =
                    u16::try_from(partition_i32).expect("Partition cast fra i32->u16 feilet");

                if get_hwm(&mut tx, version, topic, partition).await?.is_some() {
                    tracing::info!(
                        "HWM finnes allerede, hopper over bootstrap (topic: {}, partition: {})",
                        topic,
                        partition,
                    );
                    continue;
                }

                let (_, high) =
                    consumer.fetch_watermarks(topic, partition_i32, BOOTSTRAP_TIMEOUT)?;
                let offset = (high - LOOKBACK).max(DEFAULT_HWM_OFFSET);
                insert_hwm(&mut tx, version, topic, partition, offset).await?;
                tracing::info!(
                    "Setter initiell HWM (topic: {}, partition: {}, offset: {}, watermark: {})",
                    topic,
                    partition,
                    offset,
                    high,
                );
            }
        }
    }
    tx.commit().await?;

    Ok(())
}
