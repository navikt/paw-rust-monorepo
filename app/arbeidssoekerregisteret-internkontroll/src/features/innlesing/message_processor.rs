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

use crate::features::innlesing::{
    metadata::{
        meldinger::Dekodere,
        payload::PayloadStatus,
        record_metadata::{RecordMetadata, insert_record_metadata},
        signature::Signature,
        signing_keys::SigningKeyDbIds,
    },
    record::kafka_record::{insert_kafka_record, kafka_record_row, record_timestamp},
    stream_time::{get_stream_time, upsert_stream_time},
    topic_catalog_tools::{SigningRequiredCatalog, TopicCatalog},
    topic_ids::TopicIds,
};

pub struct InternkontrollMessageProcessor {
    signing_required_from: SigningRequiredCatalog,
    record_verifier: RecordVerifier,
    topic_ids: TopicIds,
    topic_catalog: TopicCatalog,
    dekodere: Dekodere,
    signing_key_id_map: SigningKeyDbIds,
}

impl InternkontrollMessageProcessor {
    pub fn new(
        signing_required_from: SigningRequiredCatalog,
        record_verifier: RecordVerifier,
        topic_ids: TopicIds,
        dekodere: Dekodere,
        topic_catalog: TopicCatalog,
        signing_key_id_map: SigningKeyDbIds,
    ) -> Self {
        Self {
            dekodere,
            signing_required_from,
            record_verifier,
            topic_ids,
            topic_catalog,
            signing_key_id_map,
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
        let (record_timestamp_type, record_timestamp_ms) = record_timestamp(msg);

        let timestamp_delta_ms = match record_timestamp_ms {
            Some(timestamp_ms) => {
                upsert_stream_time(tx, topic_id, msg.partition(), timestamp_ms).await?
            }
            None => None,
        };

        let reference_time_ms = get_stream_time(tx, topic_id, msg.partition()).await?;
        let signature = match reference_time_ms
            .and_then(|ms| DateTime::<Utc>::from_timestamp_millis(ms).map(|time| (ms, time)))
        {
            Some((reference_time_ms, reference_time)) => {
                let signature =
                    signature_from_verification(self.record_verifier.validate(reference_time, msg));
                let signing_is_requiered = self
                    .signing_required_from
                    .signing_required(msg.topic(), reference_time_ms);
                match signature {
                    Signature::Missing if !(signing_is_requiered.unwrap_or(true)) => {
                        Signature::UnsignedBeforeRequired
                    }
                    signature => signature,
                }
            }
            None => Signature::NoRecordTimestamp,
        };
        let trace_id = trace_id_from_message(msg);
        let dekodet = self.dekodere.dekod(msg).await?;
        let record_metadata_id = insert_record_metadata(
            tx,
            &self.signing_key_id_map,
            &RecordMetadata {
                topic_id,
                record_partition: msg.partition(),
                record_offset: msg.offset(),
                record_timestamp_type,
                record_timestamp_ms,
                signature,
                timestamp_delta_ms,
                trace_id,
                kilde: dekodet.as_ref().map(|melding| melding.opphav.kilde),
                system: dekodet.as_ref().map(|melding| melding.opphav.system),
                payload_status: match &dekodet {
                    Some(_) => PayloadStatus::Ok,
                    None => PayloadStatus::Ugyldig,
                },
                meldingstype: dekodet.as_ref().map(|melding| melding.meldingstype),
                periode_id: dekodet.as_ref().and_then(|melding| melding.periode_id),
                arbeidssoeker_id: dekodet
                    .as_ref()
                    .and_then(|melding| melding.arbeidssoeker_id),
                derived_at: Utc::now(),
            },
        )
        .await?;
        let backup_required = self
            .topic_catalog
            .backup_required(msg.topic())
            .ok_or(format!("Unknown topic: {}", msg.topic()))?;
        if backup_required {
            insert_kafka_record(tx, record_metadata_id, &kafka_record_row(msg)).await?;
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
