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

/// One row in `kafka_record`, borrowing key, value and headers from the message.
/// Headers are kept in Kafka order: element i of `header_keys` and
/// `header_values` is header number i.
#[derive(Clone, PartialEq, Eq)]
pub struct KafkaRecordRow<'a> {
    pub topic_id: i16,
    pub record_partition: i32,
    pub record_offset: i64,
    pub record_timestamp_type: KafkaTimestampType,
    pub record_timestamp_ms: Option<i64>,
    pub record_key: Option<&'a [u8]>,
    pub record_value: Option<&'a [u8]>,
    pub header_keys: Vec<&'a str>,
    pub header_values: Vec<Option<&'a [u8]>>,
}

impl std::fmt::Debug for KafkaRecordRow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaRecordRow")
            .field("topic_id", &self.topic_id)
            .field("record_partition", &self.record_partition)
            .field("record_offset", &self.record_offset)
            .field("record_timestamp_type", &self.record_timestamp_type)
            .field("record_timestamp_ms", &self.record_timestamp_ms)
            .finish()
    }
}

pub fn kafka_record_row(topic_id: i16, msg: &OwnedMessage) -> KafkaRecordRow<'_> {
    let (record_timestamp_type, record_timestamp_ms) = match msg.timestamp() {
        Timestamp::NotAvailable => (KafkaTimestampType::NotAvailable, None),
        Timestamp::CreateTime(ms) => (KafkaTimestampType::CreateTime, Some(ms)),
        Timestamp::LogAppendTime(ms) => (KafkaTimestampType::LogAppendTime, Some(ms)),
    };
    let (header_keys, header_values) = msg
        .headers()
        .map(|headers| headers.iter().map(|h| (h.key, h.value)).unzip())
        .unwrap_or_default();
    KafkaRecordRow {
        topic_id,
        record_partition: msg.partition(),
        record_offset: msg.offset(),
        record_timestamp_type,
        record_timestamp_ms,
        record_key: msg.key(),
        record_value: msg.payload(),
        header_keys,
        header_values,
    }
}

#[tracing::instrument(skip(conn, row))]
pub async fn insert_kafka_record(
    conn: &mut PgConnection,
    row: &KafkaRecordRow<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO kafka_record (
            topic_id, record_partition, record_offset,
            record_timestamp_type, record_timestamp_ms,
            record_key, record_value,
            header_keys, header_values
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(row.topic_id)
    .bind(row.record_partition)
    .bind(row.record_offset)
    .bind(row.record_timestamp_type)
    .bind(row.record_timestamp_ms)
    .bind(row.record_key)
    .bind(row.record_value)
    .bind(&row.header_keys)
    .bind(&row.header_values)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
