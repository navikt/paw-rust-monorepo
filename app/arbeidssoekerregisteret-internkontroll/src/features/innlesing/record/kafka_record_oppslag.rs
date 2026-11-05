use rdkafka::Timestamp;
use rdkafka::message::{Header, OwnedHeaders, OwnedMessage};
use sqlx::PgConnection;

use super::kafka_record::KafkaTimestampType;

#[derive(sqlx::FromRow)]
struct LagretRad {
    topic: String,
    record_timestamp_type: KafkaTimestampType,
    record_timestamp_ms: Option<i64>,
    record_key: Option<Vec<u8>>,
    record_value: Option<Vec<u8>>,
    header_keys: Vec<String>,
    header_values: Vec<Option<Vec<u8>>>,
}

/// Henter en lagret Kafka-melding med topic-navn og opprinnelige meldingsfelter.
#[tracing::instrument(skip(conn))]
#[allow(dead_code)]
pub async fn hent_kafka_record(
    conn: &mut PgConnection,
    topic_id: i16,
    partition: i32,
    offset: i64,
) -> Result<Option<OwnedMessage>, sqlx::Error> {
    let Some(rad) = sqlx::query_as::<_, LagretRad>(
        r#"
        SELECT t.name AS topic, m.record_timestamp_type, m.record_timestamp_ms,
               r.record_key, r.record_value, r.header_keys, r.header_values
        FROM record_metadata m
        JOIN kafka_record r ON r.metadata_id = m.id
        JOIN topic t ON t.id = m.topic_id
        WHERE m.topic_id = $1 AND m.record_partition = $2 AND m.record_offset = $3
        "#,
    )
    .bind(topic_id)
    .bind(partition)
    .bind(offset)
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(None);
    };
    let LagretRad {
        topic,
        record_timestamp_type: timestamp_type,
        record_timestamp_ms: timestamp_ms,
        record_key: key,
        record_value: payload,
        header_keys,
        header_values,
    } = rad;

    let timestamp = match (timestamp_type, timestamp_ms) {
        (KafkaTimestampType::NotAvailable, _) => Timestamp::NotAvailable,
        (KafkaTimestampType::CreateTime, Some(ms)) => Timestamp::CreateTime(ms),
        (KafkaTimestampType::LogAppendTime, Some(ms)) => Timestamp::LogAppendTime(ms),
        (KafkaTimestampType::CreateTime | KafkaTimestampType::LogAppendTime, None) => {
            Timestamp::NotAvailable
        }
    };

    let headers = if header_keys.is_empty() {
        None
    } else {
        let mut headers = OwnedHeaders::new_with_capacity(header_keys.len());
        for (key, value) in header_keys.iter().zip(&header_values) {
            headers = headers.insert(Header {
                key,
                value: value.as_deref(),
            });
        }
        Some(headers)
    };

    Ok(Some(OwnedMessage::new(
        payload, key, topic, timestamp, partition, offset, headers,
    )))
}
