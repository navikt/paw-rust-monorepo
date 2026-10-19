use std::pin::Pin;

use chrono::{DateTime, Utc};
use paw_kafka::{
    hwm::hwm_message_processor::{MessageProcessor, ProcessorError},
    signing::{KeyError, RecordVerifier, VerificationError},
};
use rdkafka::Message;
use rdkafka::message::{Headers, OwnedMessage};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::topic_ids::TopicIds;
use crate::{
    database::{
        kafka_record::{insert_kafka_record, kafka_record_row},
        record_derived::{RecordDerived, Signature, upsert_record_derived},
        stream_time::{get_stream_time, upsert_stream_time},
    },
    signing_required_tools::SigningRequiredFrom,
};

pub struct InternkontrollMessageProcessor {
    signing_required_from: SigningRequiredFrom,
    record_verifier: RecordVerifier,
    topic_ids: TopicIds,
}

impl InternkontrollMessageProcessor {
    pub fn new(
        signing_required_from: SigningRequiredFrom,
        record_verifier: RecordVerifier,
        topic_ids: TopicIds,
    ) -> Self {
        Self {
            signing_required_from,
            record_verifier,
            topic_ids,
        }
    }

    #[tracing::instrument(
        skip(self, tx, msg),
        name = "paw_internkontroll.process",
        fields(topic = msg.topic(), partition = msg.partition(), offset = msg.offset())
    )]
    pub async fn process(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        msg: &OwnedMessage,
    ) -> Result<(), ProcessorError> {
        let topic_id = self.topic_ids.get(msg.topic()).ok_or_else(|| {
            // The topic table is populated before the consumer starts. Never skip a record
            // if the lookup and subscription somehow become inconsistent.
            std::io::Error::new(std::io::ErrorKind::InvalidData, "unknown Kafka topic")
        })?;
        let row = kafka_record_row(topic_id, msg);
        insert_kafka_record(&mut **tx, &row).await?;

        let timestamp_delta_ms = match row.record_timestamp_ms {
            Some(timestamp_ms) => {
                upsert_stream_time(&mut **tx, topic_id, msg.partition(), timestamp_ms).await?
            }
            None => None,
        };

        let reference_time: Option<DateTime<Utc>> =
            get_stream_time(&mut **tx, topic_id, msg.partition())
                .await?
                .and_then(DateTime::<Utc>::from_timestamp_millis);
        let signature = match reference_time {
            Some(reference_time) => {
                signature_from_verification(self.record_verifier.validate(reference_time, msg), msg)
            }
            None => Signature::NoRecordTimestamp,
        };
        upsert_record_derived(
            &mut **tx,
            &RecordDerived {
                topic_id,
                record_partition: msg.partition(),
                record_offset: msg.offset(),
                signature,
                timestamp_delta_ms,
                trace_id: trace_id_from_message(msg),
                derived_at: Utc::now(),
            },
        )
        .await?;
        Ok(())
    }
}

// The last header wins, as in the signature verifier. Never include header values in errors.
fn last_header<'a>(msg: &'a OwnedMessage, name: &str) -> Option<&'a [u8]> {
    msg.headers()?
        .iter()
        .filter(|header| header.key == name)
        .last()
        .and_then(|header| header.value)
}

fn known_signing_key_id(msg: &OwnedMessage) -> Option<String> {
    let id = std::str::from_utf8(last_header(msg, "x-paw-signing-key-id")?).ok()?;
    (!id.is_empty() && id.len() <= 128).then(|| id.to_string())
}

fn signature_from_verification(
    result: Result<paw_kafka::signing::ValidSignature, VerificationError>,
    msg: &OwnedMessage,
) -> Signature {
    match result {
        Ok(valid) => Signature::Valid {
            signing_key_id: valid.key_id.to_string(),
        },
        Err(VerificationError::NoSignaturePresent | VerificationError::NoKeyIdPresent) => {
            Signature::Missing
        }
        Err(
            VerificationError::MalformedKeyId
            | VerificationError::Key(KeyError::UnknownKeyId { .. }),
        ) => Signature::UnknownKey,
        Err(VerificationError::SignatureMismatch | VerificationError::MalformedSignature) => {
            match known_signing_key_id(msg) {
                Some(signing_key_id) => Signature::Invalid { signing_key_id },
                None => Signature::TechnicalError {
                    signing_key_id: None,
                },
            }
        }
        // A revoked key is known, but is not allowed at the record's reference time.
        Err(VerificationError::Key(KeyError::KeyRevoked { key_id, .. })) => Signature::Invalid {
            signing_key_id: key_id.to_string(),
        },
        Err(VerificationError::Payload(_) | VerificationError::Key(_)) => {
            Signature::TechnicalError {
                signing_key_id: known_signing_key_id(msg),
            }
        }
    }
}

fn trace_id_from_message(msg: &OwnedMessage) -> Option<Uuid> {
    let value = std::str::from_utf8(last_header(msg, "traceparent")?).ok()?;
    parse_traceparent(value)
}

fn parse_traceparent(value: &str) -> Option<Uuid> {
    // W3C trace context version 00 has exactly four dash-separated fields.
    let mut fields = value.split('-');
    let (Some("00"), Some(trace), Some(parent), Some(flags), None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        return None;
    };
    if trace.len() != 32
        || parent.len() != 16
        || flags.len() != 2
        || !trace.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !parent.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !flags.bytes().all(|byte| byte.is_ascii_hexdigit())
        || parent.bytes().all(|byte| byte == b'0')
    {
        return None;
    }
    Uuid::parse_str(trace).ok().filter(|id| !id.is_nil())
}

impl MessageProcessor for InternkontrollMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(async move { self.process(tx, msg).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paw_kafka::hwm::hwm_message_processor::hwm_process_message;
    use paw_kafka::signing::PayloadError;
    use rdkafka::Timestamp;
    use rdkafka::message::{Header, OwnedHeaders};

    fn message(headers: Option<OwnedHeaders>) -> OwnedMessage {
        OwnedMessage::new(
            None,
            None,
            "topic".into(),
            Timestamp::NotAvailable,
            0,
            1,
            headers,
        )
    }

    #[test]
    fn stores_signature_outcomes_without_panicking() {
        let key = OwnedHeaders::new().insert(Header {
            key: "x-paw-signing-key-id",
            value: Some("known-key"),
        });
        let msg = message(Some(key));
        assert_eq!(
            signature_from_verification(Err(VerificationError::SignatureMismatch), &msg),
            Signature::Invalid {
                signing_key_id: "known-key".into()
            }
        );
        assert_eq!(
            signature_from_verification(Err(VerificationError::MalformedSignature), &msg),
            Signature::Invalid {
                signing_key_id: "known-key".into()
            }
        );
        assert_eq!(
            signature_from_verification(Err(VerificationError::NoSignaturePresent), &msg),
            Signature::Missing
        );
        assert_eq!(
            signature_from_verification(Err(VerificationError::MalformedKeyId), &msg),
            Signature::UnknownKey
        );
        assert_eq!(
            signature_from_verification(
                Err(VerificationError::Payload(PayloadError::TimestampMissing)),
                &msg
            ),
            Signature::TechnicalError {
                signing_key_id: Some("known-key".into())
            }
        );
    }

    #[test]
    fn trace_id_is_null_for_missing_or_invalid_traceparent() {
        let trace = "0af7651916cd43dd8448eb211c80319c";
        let valid = format!("00-{trace}-b7ad6b7169203331-01");
        let valid_message = message(Some(OwnedHeaders::new().insert(Header {
            key: "traceparent",
            value: Some(valid.as_str()),
        })));
        assert_eq!(
            trace_id_from_message(&valid_message),
            Uuid::parse_str(trace).ok()
        );
        assert_eq!(trace_id_from_message(&message(None)), None);
        for invalid in [
            "not-a-traceparent",
            "00-00000000000000000000000000000000-b7ad6b7169203331-01",
            "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01",
            "ff-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
        ] {
            let msg = message(Some(OwnedHeaders::new().insert(Header {
                key: "traceparent",
                value: Some(invalid),
            })));
            assert_eq!(trace_id_from_message(&msg), None);
        }
    }

    #[tokio::test]
    async fn raw_record_derivation_and_hwm_commit_together_or_roll_back() {
        let guard = postgres_testcontainer::postgres::setup_postgres_container()
            .await
            .unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        crate::database::topic::register_topics(pool, &["topic"])
            .await
            .unwrap();
        let topic_ids = crate::database::topic::load_topic_ids(pool).await.unwrap();
        let topic_id = topic_ids.get("topic").unwrap();
        sqlx::query("INSERT INTO hwm (version, topic, partition, hwm) VALUES (1, 'topic', 0, 0)")
            .execute(pool)
            .await
            .unwrap();

        let processor = InternkontrollMessageProcessor::new(RecordVerifier::new(), topic_ids);
        let valid = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let headers = OwnedHeaders::new()
            .insert(Header {
                key: "traceparent",
                value: Some(valid),
            })
            .insert(Header {
                key: "custom",
                value: Some("opaque"),
            });
        let first = OwnedMessage::new(
            Some(vec![0, 1, 255]),
            Some(vec![0, 7]),
            "topic".into(),
            Timestamp::CreateTime(1_000),
            0,
            1,
            Some(headers),
        );
        hwm_process_message(1, pool.clone(), &first, &processor)
            .await
            .unwrap();
        let (key, value, header_keys, header_values): (
            Option<Vec<u8>>,
            Option<Vec<u8>>,
            Vec<String>,
            Vec<Option<Vec<u8>>>,
        ) = sqlx::query_as(
            "SELECT record_key, record_value, header_keys, header_values FROM kafka_record WHERE topic_id = $1 AND record_offset = 1",
        )
        .bind(topic_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(key, Some(vec![0, 7]));
        assert_eq!(value, Some(vec![0, 1, 255]));
        assert_eq!(header_keys, vec!["traceparent", "custom"]);
        assert_eq!(header_values[1], Some(b"opaque".to_vec()));
        let (status, delta, trace): (String, Option<i64>, Option<Uuid>) = sqlx::query_as(
            "SELECT signature_status::text, timestamp_delta_ms, trace_id FROM record_derived WHERE topic_id = $1 AND record_offset = 1",
        )
        .bind(topic_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(status, "missing_signature");
        assert_eq!(delta, None);
        assert_eq!(
            trace,
            Uuid::parse_str("0af7651916cd43dd8448eb211c80319c").ok()
        );

        let invalid = OwnedHeaders::new().insert(Header {
            key: "traceparent",
            value: Some("invalid"),
        });
        let second = OwnedMessage::new(
            Some(vec![2]),
            None,
            "topic".into(),
            Timestamp::CreateTime(1_200),
            0,
            2,
            Some(invalid),
        );
        hwm_process_message(1, pool.clone(), &second, &processor)
            .await
            .unwrap();
        let (delta, trace): (Option<i64>, Option<Uuid>) = sqlx::query_as(
            "SELECT timestamp_delta_ms, trace_id FROM record_derived WHERE topic_id = $1 AND record_offset = 2",
        )
        .bind(topic_id)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(delta, Some(200));
        assert_eq!(trace, None);

        let missing_topic = InternkontrollMessageProcessor::new(
            RecordVerifier::new(),
            TopicIds::new([] as [(String, i16); 0]),
        );
        let third = OwnedMessage::new(
            None,
            None,
            "topic".into(),
            Timestamp::NotAvailable,
            0,
            3,
            None,
        );
        assert!(
            hwm_process_message(1, pool.clone(), &third, &missing_topic)
                .await
                .is_err()
        );
        let hwm: i64 = sqlx::query_scalar(
            "SELECT hwm FROM hwm WHERE version = 1 AND topic = 'topic' AND partition = 0",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(hwm, 2);
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM kafka_record WHERE topic_id = $1")
                .bind(topic_id)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(count, 2);
    }
}
