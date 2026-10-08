use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use p256::SecretKey;
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::DecodePrivateKey;
use paw_team_logs::TeamLogger;
use rdkafka::message::{Header, Headers, OwnedHeaders, ToBytes};
use rdkafka::producer::FutureRecord;

use super::wire::{last_header, signature_payload, valid_key_id};
use super::{
    LOGGER_NAME, SIGNATURE_HEADER, SIGNING_KEY_ID_HEADER, SigningError, TRACEPARENT_HEADER,
};

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
