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
        SELECT t.name AS topic, r.record_timestamp_type, r.record_timestamp_ms,
               r.record_key, r.record_value, r.header_keys, r.header_values
        FROM kafka_record r
        JOIN topic t ON t.id = r.topic_id
        WHERE r.topic_id = $1 AND r.record_partition = $2 AND r.record_offset = $3
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

#[cfg(test)]
mod tests {
    use postgres_testcontainer::postgres::setup_postgres_container;
    use rdkafka::Message;
    use rdkafka::Timestamp;
    use rdkafka::message::{Header, Headers, OwnedHeaders, OwnedMessage};

    use crate::database::kafka_record::{insert_kafka_record, kafka_record_row};
    use crate::database::topic::register_topics;

    use super::*;

    #[tokio::test]
    async fn kafka_record_oppslag_gjenoppbygger_melding_og_henter_topicnavn() {
        let postgres_guard = setup_postgres_container()
            .await
            .expect("Failed to start Postgres container");
        let pool = postgres_guard.pg_pool;
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &["test-topic"]).await.unwrap();
        let topic_id: i16 = sqlx::query_scalar("SELECT id FROM topic WHERE name = $1")
            .bind("test-topic")
            .fetch_one(&mut *conn)
            .await
            .unwrap();

        let mut headers = OwnedHeaders::new();
        headers = headers
            .insert(Header {
                key: "first",
                value: Some(b"value-1"),
            })
            .insert(Header {
                key: "second",
                value: None::<&[u8]>,
            })
            .insert(Header {
                key: "third",
                value: Some(b"value-3"),
            });
        let message = OwnedMessage::new(
            Some(b"payload".to_vec()),
            Some(b"key".to_vec()),
            "test-topic".into(),
            Timestamp::LogAppendTime(123456),
            7,
            42,
            Some(headers),
        );
        insert_kafka_record(&mut conn, &kafka_record_row(topic_id, &message))
            .await
            .unwrap();

        let stored = hent_kafka_record(&mut conn, topic_id, 7, 42)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.topic(), "test-topic");
        assert_eq!(stored.partition(), 7);
        assert_eq!(stored.offset(), 42);
        assert_eq!(stored.timestamp(), Timestamp::LogAppendTime(123456));
        assert_eq!(stored.key(), Some(b"key".as_slice()));
        assert_eq!(stored.payload(), Some(b"payload".as_slice()));
        let actual_headers: Vec<_> = stored
            .headers()
            .unwrap()
            .iter()
            .map(|header| (header.key.to_string(), header.value.map(<[u8]>::to_vec)))
            .collect();
        assert_eq!(
            actual_headers,
            vec![
                ("first".to_string(), Some(b"value-1".to_vec())),
                ("second".to_string(), None),
                ("third".to_string(), Some(b"value-3".to_vec())),
            ]
        );
        assert!(
            hent_kafka_record(&mut conn, topic_id, 7, 43)
                .await
                .unwrap()
                .is_none()
        );
    }
}
