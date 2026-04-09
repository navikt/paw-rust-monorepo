use chrono::DateTime;
use kafka_topic_backup::{KafkaMessage, prosesser_melding};
use paw_rdkafka_hwm::hwm_functions::{get_hwm, insert_hwm};
use paw_test::setup_test_db::setup_test_db;

const HWM_VERSION: i16 = 1;

#[tokio::test]
async fn alle_felt_lagres_korrekt_og_hwm_oppdateres() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let expected_topic = "paw.test-topic";
    let expected_partition = 3i32;
    let expected_offset = 42i64;
    let expected_timestamp_millis = 1_700_000_000_000i64;
    let expected_key = b"fnr-12345678901".to_vec();
    let expected_payload = br#"{"hendelse":"REGISTRERT"}"#.to_vec();
    let expected_headers = serde_json::json!({"traceparent": "00-abc", "source": "test"});

    let msg = KafkaMessage {
        topic: expected_topic.to_string(),
        partition: expected_partition,
        offset: expected_offset,
        timestamp: DateTime::from_timestamp_millis(expected_timestamp_millis).unwrap(),
        key: expected_key.clone(),
        payload: expected_payload.clone(),
        headers: Some(expected_headers.clone()),
    };

    let mut tx = pool.begin().await.unwrap();
    insert_hwm(&mut tx, HWM_VERSION, expected_topic, expected_partition, 0)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    prosesser_melding(pool.clone(), msg, HWM_VERSION)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let hwm = get_hwm(&mut tx, HWM_VERSION, expected_topic, expected_partition)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(hwm, Some(expected_offset), "HWM should be updated to the message offset");

    let lagret_melding: LagretMelding = sqlx::query_as(
        "SELECT kafka_topic, kafka_partition, kafka_offset,
                (EXTRACT(EPOCH FROM timestamp) * 1000)::BIGINT AS timestamp_millis,
                headers, record_key, record_value
         FROM data_v2
         WHERE kafka_topic = $1 AND kafka_partition = $2 AND kafka_offset = $3",
    )
    .bind(expected_topic)
    .bind(expected_partition)
    .bind(expected_offset)
    .fetch_one(&pool)
    .await
    .expect("Should find the stored record");

    assert_eq!(lagret_melding.kafka_topic, expected_topic);
    assert_eq!(lagret_melding.kafka_partition, expected_partition as i16);
    assert_eq!(lagret_melding.kafka_offset, expected_offset);
    assert_eq!(lagret_melding.timestamp_millis, expected_timestamp_millis);
    assert_eq!(lagret_melding.record_key, Some(expected_key));
    assert_eq!(lagret_melding.record_value, Some(expected_payload));
    assert_eq!(lagret_melding.headers, Some(expected_headers));
}

#[tokio::test]
async fn edgecases_lagres_korrekt() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    insert_hwm(&mut tx, HWM_VERSION, "test-topic", 0, 0)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let tom_key = KafkaMessage {
        topic: "test-topic".to_string(),
        partition: 0,
        offset: 1,
        timestamp: DateTime::from_timestamp_millis(1_000_000_000_000).unwrap(),
        key: vec![],
        payload: b"payload".to_vec(),
        headers: None,
    };
    let tom_payload = KafkaMessage {
        topic: "test-topic".to_string(),
        partition: 0,
        offset: 2,
        timestamp: DateTime::from_timestamp_millis(1_000_000_000_000).unwrap(),
        key: b"key".to_vec(),
        payload: vec![],
        headers: None,
    };
    let ingen_headers = KafkaMessage {
        topic: "test-topic".to_string(),
        partition: 0,
        offset: 3,
        timestamp: DateTime::from_timestamp_millis(1_000_000_000_000).unwrap(),
        key: b"key".to_vec(),
        payload: b"payload".to_vec(),
        headers: None,
    };

    for msg in [tom_key, tom_payload, ingen_headers] {
        prosesser_melding(pool.clone(), msg, HWM_VERSION)
            .await
            .unwrap();
    }

    let lagrede_meldinger: Vec<LagretMelding> = sqlx::query_as(
        "SELECT kafka_topic,
                    kafka_partition,
                    kafka_offset,
                    (EXTRACT(EPOCH FROM timestamp) * 1000)::BIGINT AS timestamp_millis,
                    headers,
                    record_key,
                    record_value
                FROM data_v2
                WHERE kafka_topic = $1
                ORDER BY kafka_offset",
    )
    .bind("test-topic")
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(lagrede_meldinger.len(), 3);
    assert_eq!(lagrede_meldinger[0].record_key, Some(vec![]));
    assert_eq!(lagrede_meldinger[1].record_value, Some(vec![]));
    assert_eq!(lagrede_meldinger[2].headers, None);
}

#[derive(sqlx::FromRow)]
struct LagretMelding {
    kafka_topic: String,
    kafka_partition: i16,
    kafka_offset: i64,
    timestamp_millis: i64,
    headers: Option<serde_json::Value>,
    record_key: Option<Vec<u8>>,
    record_value: Option<Vec<u8>>,
}
