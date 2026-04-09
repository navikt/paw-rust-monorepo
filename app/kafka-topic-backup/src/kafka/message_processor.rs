use crate::database::insert_data;
use crate::kafka::headers::extract_headers_as_json;
use crate::metrics;
use chrono::DateTime;
use tracing::{info, trace};
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::PgPool;
use std::error::Error;
use insert_data::insert_data;

pub async fn prosesser_melding(pg_pool: &PgPool, msg: &OwnedMessage, hwm_version: i16) -> Result<(), Box<dyn Error>> {
    let topic = msg.topic();
    let partition = msg.partition();
    let offset = msg.offset();
    let timestamp_millis = msg.timestamp().to_millis().unwrap_or(0);
    let timestamp = DateTime::from_timestamp_millis(timestamp_millis)
        .ok_or_else(|| format!("Invalid timestamp: {}", timestamp_millis))?;

    let mut tx = pg_pool.begin().await?;
    let hwm_ok = update_hwm(&mut tx, hwm_version, topic, partition, offset).await?;

    if hwm_ok {
        insert_data(
            &mut tx,
            topic,
            partition,
            offset,
            timestamp,
            extract_headers_as_json(msg),
            msg.key().unwrap_or(&[]).to_vec(),
            msg.payload().unwrap_or(&[]).to_vec(),
        )
        .await?;
        tx.commit().await?;
        trace!("Message processed: topic={}, partition={}, offset={}", topic, partition, offset);
    } else {
        info!("Below HWM, skipping insert: topic={}, partition={}, offset={}", topic, partition, offset);
        tx.rollback().await?;
    }
    metrics::increment_kafka_messages_processed(hwm_ok, topic, partition);
    Ok(())
}
