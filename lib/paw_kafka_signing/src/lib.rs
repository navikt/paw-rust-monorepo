//! Kotlin-compatible P-256/SHA-256 Kafka record signing and observational validation.
//! Validation never filters Kafka records. Warnings go only to private Team Logs.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
};
use p256::SecretKey;
use p256::ecdsa::{
    Signature, SigningKey, VerifyingKey,
    signature::{Signer, Verifier},
};
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey};
use paw_team_logs::{TeamLogger, TeamLogsError};
use rdkafka::Message;
use rdkafka::message::{Header, Headers, OwnedHeaders, ToBytes};
use rdkafka::producer::FutureRecord;
use thiserror::Error;

pub const SIGNATURE_HEADER: &str = "x-paw-signature";
pub const SIGNING_KEY_ID_HEADER: &str = "x-paw-signing-key-id";
const TRACEPARENT_HEADER: &str = "traceparent";
const LOGGER_NAME: &str = "team-logs-logger";

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("invalid P-256 private key")]
    PrivateKey,
    #[error("invalid P-256 public key")]
    PublicKey,
    #[error("invalid Base64-encoded key")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid key ID")]
    KeyId,
    #[error("signed field exceeds the 32-bit length limit")]
    FieldTooLong,
    #[error("could not determine record timestamp")]
    Timestamp,
    #[error("could not read key file: {0}")]
    Io(#[from] std::io::Error),
}

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

/// Keys are read once at startup; the private key is never logged or embedded in the crate.
pub struct RecordSigner {
    key_id: String,
    key: SigningKey,
}

impl RecordSigner {
    /// Parses a Base64 PKCS#8 DER private key from a mounted secret.
    pub fn from_pkcs8_base64(
        key_id: impl Into<String>,
        encoded: &str,
    ) -> Result<Self, SigningError> {
        let key_id = key_id.into();
        if !valid_key_id(&key_id) {
            return Err(SigningError::KeyId);
        }
        let der = decode_private_key(encoded)?;
        let secret = SecretKey::from_pkcs8_der(&der).map_err(|_| SigningError::PrivateKey)?;
        Ok(Self {
            key_id,
            key: SigningKey::from(secret),
        })
    }

    /// Reads `PAW_SIGNING_PRIVATE_KEY_PKCS8_BASE64` and `PAW_SIGNING_KEY_ID`
    /// from files in the Nais-mounted secret directory.
    pub fn from_secret_dir(dir: &Path) -> Result<Self, SigningError> {
        Self::from_pkcs8_base64(
            fs::read_to_string(dir.join("PAW_SIGNING_KEY_ID"))?.trim(),
            &fs::read_to_string(dir.join("PAW_SIGNING_PRIVATE_KEY_PKCS8_BASE64"))?,
        )
    }

    /// Produces DER-encoded ECDSA signature bytes for the Kotlin wire format.
    pub fn sign(
        &self,
        key: &[u8],
        traceparent: &[u8],
        timestamp_ms: i64,
        value: &[u8],
    ) -> Result<Vec<u8>, SigningError> {
        let payload = signature_payload(key, traceparent, timestamp_ms, value)?;
        let signature: Signature = self.key.sign(&payload);
        Ok(signature.to_der().as_bytes().to_vec())
    }

    /// Signs already-serialized Kafka bytes and fixes the timestamp before sending.
    /// On an error, any stale signing headers are removed before returning the record.
    pub fn sign_record<'a, K: ToBytes + ?Sized, P: ToBytes + ?Sized>(
        &self,
        mut record: FutureRecord<'a, K, P>,
    ) -> (FutureRecord<'a, K, P>, Result<(), SigningError>) {
        let result = self.sign_record_inner(&mut record);
        if result.is_err() {
            record.headers = Some(strip_signing_headers(record.headers.as_ref()));
        }
        (record, result)
    }

    fn sign_record_inner<K: ToBytes + ?Sized, P: ToBytes + ?Sized>(
        &self,
        record: &mut FutureRecord<'_, K, P>,
    ) -> Result<(), SigningError> {
        let timestamp = match record.timestamp {
            Some(value) => value,
            None => i64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| SigningError::Timestamp)?
                    .as_millis(),
            )
            .map_err(|_| SigningError::Timestamp)?,
        };
        let traceparent =
            last_header(record.headers.as_ref(), TRACEPARENT_HEADER).unwrap_or_default();
        let signature = self.sign(
            record.key.map_or(&[], ToBytes::to_bytes),
            traceparent,
            timestamp,
            record.payload.map_or(&[], ToBytes::to_bytes),
        )?;
        let headers = strip_signing_headers(record.headers.as_ref());
        record.headers = Some(
            headers
                .insert(Header {
                    key: SIGNATURE_HEADER,
                    value: Some(URL_SAFE_NO_PAD.encode(signature).as_bytes()),
                })
                .insert(Header {
                    key: SIGNING_KEY_ID_HEADER,
                    value: Some(self.key_id.as_bytes()),
                }),
        );
        record.timestamp = Some(timestamp);
        Ok(())
    }

    /// Kotlin-compatible failure policy: warning in Team Logs, record sent unsigned.
    pub fn sign_or_warn<'a, K: ToBytes + ?Sized, P: ToBytes + ?Sized>(
        &self,
        record: FutureRecord<'a, K, P>,
        logs: &impl TeamLogger,
    ) -> FutureRecord<'a, K, P> {
        let topic = record.topic.to_owned();
        let (record, result) = self.sign_record(record);
        if let Err(error) = result {
            let _ = logs.error(LOGGER_NAME, &format!(
                "[kafka-signing] Failed to sign Kafka record on topic={topic}, record will be sent unsigned: {error}"
            ));
        }
        record
    }
}

fn decode_private_key(input: &str) -> Result<Vec<u8>, SigningError> {
    // Kotlin accepts PEM wrappers, whitespace, and either Base64 alphabet.
    let cleaned: String = input
        .lines()
        .filter(|line| !line.trim_start().starts_with("-----"))
        .flat_map(str::chars)
        .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '-' || *c == '_')
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();
    let padded = format!(
        "{cleaned}{:=>width$}",
        "",
        width = (4 - cleaned.len() % 4) % 4
    );
    Ok(STANDARD.decode(padded)?)
}

/// Copies headers in order, excluding prior signatures when forwarding a record.
pub fn strip_signing_headers(headers: Option<&OwnedHeaders>) -> OwnedHeaders {
    let mut result = OwnedHeaders::new();
    if let Some(headers) = headers {
        for header in headers.iter() {
            if header.key != SIGNATURE_HEADER && header.key != SIGNING_KEY_ID_HEADER {
                result = result.insert(Header {
                    key: header.key,
                    value: header.value,
                });
            }
        }
    }
    result
}

fn last_header<'a, H: Headers>(headers: Option<&'a H>, name: &str) -> Option<&'a [u8]> {
    headers?
        .iter()
        .filter(|h| h.key == name)
        .last()
        .and_then(|h| h.value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validation {
    Valid,
    MissingHeaders { signature: bool, key_id: bool },
    UnknownKey(String),
    InvalidSignature(String),
    TechnicalError,
}

#[derive(Default)]
pub struct RecordVerifier {
    keys: HashMap<String, VerifyingKey>,
}

impl RecordVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an X.509/SPKI DER public key encoded as standard Base64.
    pub fn add_key(
        &mut self,
        key_id: impl Into<String>,
        encoded: &str,
    ) -> Result<(), SigningError> {
        let key_id = key_id.into();
        if !valid_key_id(&key_id) || self.keys.contains_key(&key_id) {
            return Err(SigningError::KeyId);
        }
        let der = STANDARD.decode(encoded.trim())?;
        let key = VerifyingKey::from_public_key_der(&der).map_err(|_| SigningError::PublicKey)?;
        self.keys.insert(key_id, key);
        Ok(())
    }

    /// Loads the same `index` and `<key-id>.pub.b64` directory used by Kotlin.
    /// Call at startup; missing/invalid entries must not silently disable validation.
    pub fn from_key_directory(dir: &Path) -> Result<Self, SigningError> {
        let index = fs::read_to_string(dir.join("index"))?;
        let mut verifier = Self::new();
        for key_id in index.lines().map(str::trim).filter(|id| !id.is_empty()) {
            if !valid_key_id(key_id) {
                return Err(SigningError::KeyId);
            }
            verifier.add_key(
                key_id,
                &fs::read_to_string(dir.join(format!("{key_id}.pub.b64")))?,
            )?;
        }
        if verifier.keys.is_empty() {
            return Err(SigningError::KeyId);
        }
        Ok(verifier)
    }

    /// Examines raw Kafka bytes without modifying or consuming the record.
    pub fn validate<M: Message>(&self, message: &M) -> Validation {
        let signature = last_header(message.headers(), SIGNATURE_HEADER);
        let key_id = last_header(message.headers(), SIGNING_KEY_ID_HEADER);
        let (Some(signature), Some(key_id)) = (signature, key_id) else {
            return Validation::MissingHeaders {
                signature: signature.is_some(),
                key_id: key_id.is_some(),
            };
        };
        let Ok(key_id) = std::str::from_utf8(key_id) else {
            return Validation::TechnicalError;
        };
        let Some(key) = self.keys.get(key_id) else {
            return Validation::UnknownKey(key_id.to_string());
        };
        let Ok(encoded_signature) = std::str::from_utf8(signature) else {
            return Validation::TechnicalError;
        };
        let Ok(signature) = URL_SAFE_NO_PAD
            .decode(encoded_signature)
            .or_else(|_| URL_SAFE.decode(encoded_signature))
        else {
            return Validation::TechnicalError;
        };
        let Ok(signature) = Signature::from_der(&signature) else {
            return Validation::TechnicalError;
        };
        // Kotlin's ConsumerRecord.timestamp() returns -1 when no timestamp exists.
        let timestamp = message.timestamp().to_millis().unwrap_or(-1);
        let traceparent = last_header(message.headers(), TRACEPARENT_HEADER).unwrap_or_default();
        let Ok(payload) = signature_payload(
            message.key().unwrap_or_default(),
            traceparent,
            timestamp,
            message.payload().unwrap_or_default(),
        ) else {
            return Validation::TechnicalError;
        };
        if key.verify(&payload, &signature).is_ok() {
            Validation::Valid
        } else {
            Validation::InvalidSignature(key_id.to_string())
        }
    }

    /// Warns only in Team Logs. A failed log send is reported separately and never filters the message.
    pub fn validate_and_warn<M: Message>(
        &self,
        message: &M,
        logs: &impl TeamLogger,
    ) -> (Validation, Option<TeamLogsError>) {
        let result = self.validate(message);
        let Some(warning) = warning_message(&result, message) else {
            return (result, None);
        };
        (result, logs.warn(LOGGER_NAME, &warning).err())
    }
}

fn warning_message<M: Message>(result: &Validation, message: &M) -> Option<String> {
    let topic = message.topic();
    let partition = message.partition();
    let offset = message.offset();
    let traceparent = last_header(message.headers(), TRACEPARENT_HEADER)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .filter(|value| {
            value.len() == 55 && value.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
        })
        .unwrap_or("ukjent");
    let trace_id = traceparent.split('-').nth(1).unwrap_or("ukjent");
    let warning = match &result {
        Validation::Valid => return None,
        Validation::MissingHeaders { signature, key_id } => format!(
            "[kafka-signing] Mangler signaturheader(er) — topic={topic}, partition={partition}, offset={offset}, harSignatur={signature}, harNøkkelId={key_id}, trace_id={trace_id}, traceparent={traceparent}"
        ),
        Validation::UnknownKey(key_id) => {
            let key_id = if valid_key_id(key_id) {
                key_id.as_str()
            } else {
                "ukjent"
            };
            format!(
                "[kafka-signing] Ukjent signeringsnøkkel-id='{key_id}' — topic={topic}, partition={partition}, offset={offset}, trace_id={trace_id}, traceparent={traceparent}"
            )
        }
        Validation::InvalidSignature(key_id) => {
            let key_id = if valid_key_id(key_id) {
                key_id.as_str()
            } else {
                "ukjent"
            };
            format!(
                "[kafka-signing] Ugyldig signatur — topic={topic}, partition={partition}, offset={offset}, keyId='{key_id}', trace_id={trace_id}, traceparent={traceparent}"
            )
        }
        Validation::TechnicalError => format!(
            "[kafka-signing] Teknisk feil ved signaturvalidering — topic={topic}, partition={partition}, offset={offset}, trace_id={trace_id}, traceparent={traceparent}"
        ),
    };
    Some(warning)
}

fn valid_key_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::message::{Header, OwnedMessage, Timestamp};

    #[test]
    fn warning_text_matches_kotlin_alarm_message() {
        let record = OwnedMessage::new(
            None,
            None,
            "topic".into(),
            Timestamp::CreateTime(1),
            2,
            42,
            None,
        );
        assert_eq!(
            warning_message(
                &Validation::MissingHeaders {
                    signature: false,
                    key_id: false
                },
                &record
            )
            .unwrap(),
            "[kafka-signing] Mangler signaturheader(er) — topic=topic, partition=2, offset=42, harSignatur=false, harNøkkelId=false, trace_id=ukjent, traceparent=ukjent"
        );
        assert_eq!(
            warning_message(&Validation::UnknownKey("key-id".into()), &record).unwrap(),
            "[kafka-signing] Ukjent signeringsnøkkel-id='key-id' — topic=topic, partition=2, offset=42, trace_id=ukjent, traceparent=ukjent"
        );
        assert_eq!(
            warning_message(&Validation::InvalidSignature("key-id".into()), &record).unwrap(),
            "[kafka-signing] Ugyldig signatur — topic=topic, partition=2, offset=42, keyId='key-id', trace_id=ukjent, traceparent=ukjent"
        );
        assert!(warning_message(&Validation::Valid, &record).is_none());
    }

    #[test]
    fn does_not_copy_untrusted_trace_header_into_alarm() {
        let record = OwnedMessage::new(
            None,
            None,
            "topic".into(),
            Timestamp::CreateTime(1),
            2,
            42,
            Some(OwnedHeaders::new().insert(Header {
                key: TRACEPARENT_HEADER,
                value: Some("hello\nsecret"),
            })),
        );
        let line = warning_message(&Validation::TechnicalError, &record).unwrap();
        assert!(line.contains("traceparent=ukjent"));
        assert!(!line.contains("secret"));
    }
}
