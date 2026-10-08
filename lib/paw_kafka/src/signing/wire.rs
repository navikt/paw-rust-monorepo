use super::SigningError;
use rdkafka::message::Headers;

/// The signed representation is u32 BE length + key, u32 BE length + traceparent,
/// i64 BE timestamp in milliseconds, u32 BE length + value. Missing fields are empty.
pub fn signature_payload(
    key: &[u8],
    traceparent: &[u8],
    timestamp_ms: i64,
    value: &[u8],
) -> Result<Vec<u8>, SigningError> {
    let capacity = 4usize
        .checked_add(key.len())
        .and_then(|n| n.checked_add(4 + 8 + 4))
        .and_then(|n| n.checked_add(traceparent.len()))
        .and_then(|n| n.checked_add(value.len()))
        .ok_or(SigningError::FieldTooLong)?;
    let mut bytes = Vec::with_capacity(capacity);
    for field in [key, traceparent] {
        bytes.extend_from_slice(
            &u32::try_from(field.len())
                .map_err(|_| SigningError::FieldTooLong)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(field);
    }
    bytes.extend_from_slice(&timestamp_ms.to_be_bytes());
    bytes.extend_from_slice(
        &u32::try_from(value.len())
            .map_err(|_| SigningError::FieldTooLong)?
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

pub(crate) fn valid_key_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
