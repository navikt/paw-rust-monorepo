use rdkafka::{
    message::{Headers, ToBytes},
    producer::FutureRecord,
};

use crate::signing::{PayloadError, TRACEPARENT_HEADER};

pub fn signature_payload_from_message<M: rdkafka::Message>(
    message: &M,
) -> Result<Vec<u8>, PayloadError> {
    let record_key = message.key().map_or(&[] as &[u8], ToBytes::to_bytes);
    let traceparent = last_header(message.headers(), TRACEPARENT_HEADER).unwrap_or_default();
    let timestamp = message
        .timestamp()
        .to_millis()
        .ok_or(PayloadError::TimestampMissing)?;
    let record_value = message.payload().map_or(&[] as &[u8], ToBytes::to_bytes);
    signature_payload(record_key, traceparent, timestamp, record_value)
}

pub fn signature_payload_from_furture_record<K: ToBytes + ?Sized, P: ToBytes + ?Sized>(
    record: &FutureRecord<'_, K, P>,
) -> Result<Vec<u8>, PayloadError> {
    let record_key = record.key.map_or(&[] as &[u8], ToBytes::to_bytes);
    let traceparent = last_header(record.headers.as_ref(), TRACEPARENT_HEADER).unwrap_or_default();
    let Some(timestamp) = record.timestamp else {
        return Err(PayloadError::TimestampMissing);
    };
    let record_value = record.payload.map_or(&[] as &[u8], ToBytes::to_bytes);
    signature_payload(record_key, traceparent, timestamp, record_value)
}

/// The signed representation is u32 BE length + key, u32 BE length + traceparent,
/// i64 BE timestamp in milliseconds, u32 BE length + value. Missing fields are empty.
pub fn signature_payload(
    key: &[u8],
    traceparent: &[u8],
    timestamp_ms: i64,
    value: &[u8],
) -> Result<Vec<u8>, PayloadError> {
    let capacity = 4usize
        .checked_add(key.len())
        .and_then(|n| n.checked_add(4 + 8 + 4))
        .and_then(|n| n.checked_add(traceparent.len()))
        .and_then(|n| n.checked_add(value.len()))
        .ok_or(PayloadError::ToManyBytes)?;
    let mut bytes = Vec::with_capacity(capacity);
    for field in [key, traceparent] {
        bytes.extend_from_slice(
            &u32::try_from(field.len())
                .map_err(|_| PayloadError::ToManyBytes)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(field);
    }
    bytes.extend_from_slice(&timestamp_ms.to_be_bytes());
    bytes.extend_from_slice(
        &u32::try_from(value.len())
            .map_err(|_| PayloadError::ToManyBytes)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(value);
    Ok(bytes)
}

pub(crate) fn last_header<'a, H: Headers>(headers: Option<&'a H>, name: &str) -> Option<&'a [u8]> {
    headers?
        .iter()
        .filter(|h| h.key == name)
        .last()
        .and_then(|h| h.value)
}
