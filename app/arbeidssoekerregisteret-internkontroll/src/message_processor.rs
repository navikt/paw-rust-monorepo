use std::pin::Pin;

use chrono::{DateTime, Utc};
use paw_kafka::{
    hwm::hwm_message_processor::{MessageProcessor, ProcessorError},
    signing::{KeyError, PayloadError, RecordVerifier, VerificationError},
};
use rdkafka::Message;
use rdkafka::message::{Headers, OwnedMessage};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::topic_ids::TopicIds;
use crate::{
    database::{
        kafka_record::{insert_kafka_record, kafka_record_row},
        record_derived::{RecordDerived, upsert_record_derived},
        signature::Signature,
        stream_time::{get_stream_time, upsert_stream_time},
        trace::upsert_trace,
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

        let reference_time_ms = get_stream_time(&mut **tx, topic_id, msg.partition()).await?;
        let signature = match reference_time_ms
            .and_then(|ms| DateTime::<Utc>::from_timestamp_millis(ms).map(|time| (ms, time)))
        {
            Some((reference_time_ms, reference_time)) => apply_signing_requirement(
                signature_from_verification(self.record_verifier.validate(reference_time, msg)),
                reference_time_ms,
                self.signing_required_from.get(msg.topic()),
            ),
            None => Signature::NoRecordTimestamp,
        };
        let trace_id = trace_id_from_message(msg);
        let sequence_number = upsert_record_derived(
            &mut **tx,
            &RecordDerived {
                topic_id,
                record_partition: msg.partition(),
                record_offset: msg.offset(),
                signature,
                timestamp_delta_ms,
                trace_id,
                derived_at: Utc::now(),
            },
        )
        .await?;
        if let (Some(trace_id), Some(reference_time_ms)) = (trace_id, reference_time_ms) {
            upsert_trace(&mut **tx, trace_id, reference_time_ms, sequence_number).await?;
        }
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

fn signature_from_verification(
    result: Result<paw_kafka::signing::ValidSignature, VerificationError>,
) -> Signature {
    let key = |key_id: &str| key_id.to_string();
    match result {
        Ok(valid) => Signature::Valid {
            signing_key_id: key(valid.key_id),
        },
        Err(VerificationError::SignatureMismatch { key_id }) => Signature::CryptographicMismatch {
            signing_key_id: key(key_id),
        },
        Err(VerificationError::MalformedSignature { key_id }) => Signature::Malformed {
            signing_key_id: key(key_id),
        },
        Err(VerificationError::Key(KeyError::KeyRevoked { key_id, .. })) => Signature::KeyRevoked {
            signing_key_id: key(key_id),
        },
        Err(VerificationError::Payload {
            key_id,
            error: PayloadError::ToManyBytes,
        }) => Signature::PayloadTooLarge {
            signing_key_id: key(key_id),
        },
        Err(VerificationError::Payload {
            error: PayloadError::TimestampMissing,
            ..
        }) => Signature::NoRecordTimestamp,
        Err(VerificationError::NoSignaturePresent | VerificationError::NoKeyIdPresent) => {
            Signature::Missing
        }
        Err(
            VerificationError::MalformedKeyId
            | VerificationError::Key(KeyError::UnknownKeyId { .. }),
        ) => Signature::UnknownKey,
        Err(VerificationError::Key(
            KeyError::MalformedKey
            | KeyError::InvalidRevocationDateTime { .. }
            | KeyError::IO(_)
            | KeyError::DuplicateKeyId { .. },
        )) => Signature::TechnicalError,
    }
}

fn apply_signing_requirement(
    signature: Signature,
    reference_time_ms: i64,
    signing_required_from_ms: i64,
) -> Signature {
    match signature {
        Signature::Missing if reference_time_ms < signing_required_from_ms => {
            Signature::UnsignedBeforeRequired
        }
        signature => signature,
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
    use paw_kafka::signing::ValidSignature;
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

    const KEY: &str = "key-a";

    fn no_requirement() -> SigningRequiredFrom {
        SigningRequiredFrom::new(i64::MAX, [] as [(String, i64); 0])
    }

    fn t() -> DateTime<Utc> {
        DateTime::from_timestamp_millis(0).unwrap()
    }

    #[test]
    fn ok_gir_valid() {
        assert_eq!(
            signature_from_verification(Ok(ValidSignature { key_id: KEY })),
            Signature::Valid {
                signing_key_id: KEY.into()
            }
        );
    }

    #[test]
    fn manglende_signatur_gir_missing() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::NoSignaturePresent)),
            Signature::Missing
        );
    }

    #[test]
    fn manglende_nokkel_id_gir_missing() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::NoKeyIdPresent)),
            Signature::Missing
        );
    }

    #[test]
    fn ugyldig_nokkel_id_gir_unknown_key() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::MalformedKeyId)),
            Signature::UnknownKey
        );
    }

    #[test]
    fn ukjent_nokkel_gir_unknown_key() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::Key(KeyError::UnknownKeyId {
                key_id: "x".into()
            }))),
            Signature::UnknownKey
        );
    }

    #[test]
    fn ugyldig_signatur_gir_malformed_signature() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::MalformedSignature { key_id: KEY })),
            Signature::Malformed {
                signing_key_id: KEY.into()
            }
        );
    }

    #[test]
    fn feil_signatur_gir_signature_mismatch() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::SignatureMismatch { key_id: KEY })),
            Signature::CryptographicMismatch {
                signing_key_id: KEY.into()
            }
        );
    }

    #[test]
    fn tilbakekalt_nokkel_gir_key_revoked() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::Key(KeyError::KeyRevoked {
                key_id: KEY,
                not_valid_after: t(),
                recorded_revocation: t(),
                reference_time: t()
            }))),
            Signature::KeyRevoked {
                signing_key_id: KEY.into()
            }
        );
    }

    #[test]
    fn for_stor_payload_gir_payload_too_large() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::Payload {
                key_id: KEY,
                error: PayloadError::ToManyBytes
            })),
            Signature::PayloadTooLarge {
                signing_key_id: KEY.into()
            }
        );
    }

    #[test]
    fn manglende_tidsstempel_gir_no_record_timestamp() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::Payload {
                key_id: KEY,
                error: PayloadError::TimestampMissing
            })),
            Signature::NoRecordTimestamp
        );
    }

    #[test]
    fn ugyldig_nokkel_gir_technical_error() {
        assert_eq!(
            signature_from_verification(Err(VerificationError::Key(KeyError::MalformedKey))),
            Signature::TechnicalError
        );
    }

    #[test]
    fn manglende_signatur_for_kravet_gir_unsigned_before_required() {
        assert_eq!(
            apply_signing_requirement(Signature::Missing, 999, 1_000),
            Signature::UnsignedBeforeRequired
        );
    }

    #[test]
    fn manglende_signatur_fra_kravet_gir_missing() {
        assert_eq!(
            apply_signing_requirement(Signature::Missing, 1_000, 1_000),
            Signature::Missing
        );
    }

    #[test]
    fn andre_statuser_endres_ikke_for_kravet() {
        for signature in crate::database::signature::tests::alle_signaturer()
            .into_iter()
            .filter(|signature| *signature != Signature::Missing)
        {
            assert_eq!(
                apply_signing_requirement(signature.clone(), 0, 1_000),
                signature
            );
        }
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

        let processor =
            InternkontrollMessageProcessor::new(no_requirement(), RecordVerifier::new(), topic_ids);
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
        assert_eq!(status, "unsigned_before_required");
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
        let traces: Vec<(Uuid, i64, i64)> =
            sqlx::query_as("SELECT trace_id, first_reference_ms, last_reference_ms FROM trace")
                .fetch_all(pool)
                .await
                .unwrap();
        assert_eq!(
            traces,
            vec![(
                Uuid::parse_str("0af7651916cd43dd8448eb211c80319c").unwrap(),
                1_000,
                1_000
            )]
        );

        let missing_topic = InternkontrollMessageProcessor::new(
            no_requirement(),
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
