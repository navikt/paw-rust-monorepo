use crate::features::innlesing::metadata::record_metadata_id::RecordMetadataId;
use rdkafka::Message;
use rdkafka::Timestamp;
use rdkafka::message::{Headers, OwnedMessage};
use sqlx::PgConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "kafka_timestamp_type", rename_all = "snake_case")]
pub enum KafkaTimestampType {
    NotAvailable,
    CreateTime,
    LogAppendTime,
}

/// Timestamp type and value as Kafka reported them.
pub fn record_timestamp(msg: &OwnedMessage) -> (KafkaTimestampType, Option<i64>) {
    match msg.timestamp() {
        Timestamp::NotAvailable => (KafkaTimestampType::NotAvailable, None),
        Timestamp::CreateTime(ms) => (KafkaTimestampType::CreateTime, Some(ms)),
        Timestamp::LogAppendTime(ms) => (KafkaTimestampType::LogAppendTime, Some(ms)),
    }
}

/// One row in `kafka_record`, borrowing key, value and headers from the message.
/// Topic, partition, offset and timestamp live in `record_metadata`.
/// Headers are kept in Kafka order: element i of `header_keys` and
/// `header_values` is header number i.
#[derive(Clone, PartialEq, Eq)]
pub struct KafkaRecordRow<'a> {
    pub record_key: Option<&'a [u8]>,
    pub record_value: Option<&'a [u8]>,
    pub header_keys: Vec<&'a str>,
    pub header_values: Vec<Option<&'a [u8]>>,
}

impl std::fmt::Debug for KafkaRecordRow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaRecordRow").finish_non_exhaustive()
    }
}

pub fn kafka_record_row(msg: &OwnedMessage) -> KafkaRecordRow<'_> {
    let (header_keys, header_values) = msg
        .headers()
        .map(|headers| headers.iter().map(|h| (h.key, h.value)).unzip())
        .unwrap_or_default();
    KafkaRecordRow {
        record_key: msg.key(),
        record_value: msg.payload(),
        header_keys,
        header_values,
    }
}

/// Stores the raw record for an existing `record_metadata` row.
#[tracing::instrument(skip(conn, row))]
pub async fn insert_kafka_record(
    conn: &mut PgConnection,
    record_metadata_id: RecordMetadataId,
    row: &KafkaRecordRow<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO kafka_record (
            metadata_id, record_key, record_value, header_keys, header_values
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(record_metadata_id)
    .bind(row.record_key)
    .bind(row.record_value)
    .bind(&row.header_keys)
    .bind(&row.header_values)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
