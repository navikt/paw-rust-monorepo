use super::warning::warning_message;
use super::wire::{last_header, signature_payload, valid_key_id};
use super::{
    LOGGER_NAME, SIGNATURE_HEADER, SIGNING_KEY_ID_HEADER, SigningError, TRACEPARENT_HEADER,
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use p256::pkcs8::DecodePublicKey;
use paw_team_logs::{TeamLogger, TeamLogsError};
use rdkafka::Message;
use std::collections::HashMap;

/// A signature verified with the public key identified by `key_id`.
/// Whether this key was permitted at the record's offset is up to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidSignature {
    pub key_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    MissingSignature,
    /// The ID is an untrusted claim from the record's header.
    UnknownKey {
        key_id: String,
    },
    /// The ID is an untrusted claim from the record's header.
    InvalidSignature {
        key_id: String,
    },
    /// A controlled reason, never raw record contents or parser output.
    TechnicalError {
        reason: &'static str,
    },
}

// Keep retired keys here for historical verification. The caller decides when
// each key was valid, using topic/partition/offset rather than record timestamp.
// Source: paw-arbeidssoekerregisteret-monorepo-intern and -ekstern,
// lib/kafka-signing/src/main/resources/paw-signing-public-keys/.
enum EmbeddedKey {
    Active {
        id: &'static str,
        encoded: &'static str,
    },
    // Revocation is metadata only. Verification still accepts this public key;
    // the caller must decide whether it was allowed for a particular record.
    #[allow(dead_code, reason = "No public keys are revoked yet")]
    Revoked {
        id: &'static str,
        encoded: &'static str,
        revoked_after: DateTime<Utc>,
        recorded_at: DateTime<Utc>,
    },
}

impl EmbeddedKey {
    fn id(&self) -> &'static str {
        match self {
            Self::Active { id, .. } | Self::Revoked { id, .. } => id,
        }
    }

    fn encoded(&self) -> &'static str {
        match self {
            Self::Active { encoded, .. } | Self::Revoked { encoded, .. } => encoded,
        }
    }

    #[allow(dead_code, reason = "Used when a key is marked revoked in the catalog")]
    fn revoked(
        id: &'static str,
        encoded: &'static str,
        revoked_after: &str,
        recorded_at: &str,
    ) -> Result<Self, SigningError> {
        let parse = |timestamp: &str| {
            DateTime::parse_from_rfc3339(timestamp)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|_| SigningError::InvalidRevocationDate)
        };
        Ok(Self::Revoked {
            id,
            encoded,
            revoked_after: parse(revoked_after)?,
            recorded_at: parse(recorded_at)?,
        })
    }
}

macro_rules! embedded_key {
    ($id:literal) => {
        EmbeddedKey::Active {
            id: $id,
            encoded: include_str!(concat!("../../public-keys/", $id, ".pub.b64")),
        }
    };
    (revoked $id:literal, $revoked_after:literal, $recorded_at:literal) => {
        EmbeddedKey::revoked(
            $id,
            include_str!(concat!("../../public-keys/", $id, ".pub.b64")),
            $revoked_after,
            $recorded_at,
        )?
    };
}

fn embedded_keys() -> Result<Vec<EmbeddedKey>, SigningError> {
    Ok(vec![
        embedded_key!("dev-paw-api-bekreftelse-ecdsa-v1"),
        embedded_key!("dev-paw-bekreftelse-filter-ecdsa-v1"),
        embedded_key!("dev-paw-bekreftelse-tjeneste-ecdsa-v1"),
        embedded_key!("dev-paw-bekreftelse-utgang-ecdsa-v1"),
        embedded_key!("dev-paw-egenvurdering-api-ecdsa-v1"),
        embedded_key!("dev-paw-event-processor-ecdsa-v1"),
        embedded_key!("dev-paw-profilering-ecdsa-v1"),
        embedded_key!("paw-api-inngang-kafka-signing-key"),
        embedded_key!("paw-api-inngang-kafka-signing-key-v2"),
        embedded_key!("prod-paw-api-bekreftelse-ecdsa-v1"),
        embedded_key!("prod-paw-api-inngang-ecdsa-v1"),
        embedded_key!("prod-paw-bekreftelse-filter-ecdsa-v1"),
        embedded_key!("prod-paw-bekreftelse-tjeneste-ecdsa-v1"),
        embedded_key!("prod-paw-bekreftelse-utgang-ecdsa-v1"),
        embedded_key!("prod-paw-egenvurdering-api-ecdsa-v1"),
        embedded_key!("prod-paw-event-processor-ecdsa-v1"),
        embedded_key!("prod-paw-profilering-ecdsa-v1"),
    ])
}

#[derive(Default)]
pub struct RecordVerifier {
    keys: HashMap<String, VerifyingKey>,
}

impl RecordVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads all known public keys, including old keys for historical records.
    /// Fails at startup if any embedded key is invalid.
    pub fn from_embedded_keys() -> Result<Self, SigningError> {
        let mut verifier = Self::new();
        for key in embedded_keys()? {
            verifier.add_key(key.id(), key.encoded())?;
        }
        Ok(verifier)
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

    /// Examines raw Kafka bytes without modifying or consuming the record.
    /// A valid signature does not imply the key was allowed at this offset.
    pub fn validate<M: Message>(&self, message: &M) -> Result<ValidSignature, SignatureError> {
        let signature = last_header(message.headers(), SIGNATURE_HEADER);
        let key_id = last_header(message.headers(), SIGNING_KEY_ID_HEADER);
        let Some(signature) = signature else {
            return Err(SignatureError::MissingSignature);
        };
        let Some(key_id) = key_id else {
            return Err(SignatureError::TechnicalError {
                reason: "missing key ID",
            });
        };
        let Ok(key_id) = std::str::from_utf8(key_id) else {
            return Err(SignatureError::TechnicalError {
                reason: "invalid key ID encoding",
            });
        };
        if !valid_key_id(key_id) {
            return Err(SignatureError::TechnicalError {
                reason: "invalid key ID",
            });
        }
        let Some(key) = self.keys.get(key_id) else {
            return Err(SignatureError::UnknownKey {
                key_id: key_id.to_string(),
            });
        };
        let Ok(encoded_signature) = std::str::from_utf8(signature) else {
            return Err(SignatureError::TechnicalError {
                reason: "invalid signature encoding",
            });
        };
        let Ok(signature) = URL_SAFE_NO_PAD
            .decode(encoded_signature)
            .or_else(|_| URL_SAFE.decode(encoded_signature))
        else {
            return Err(SignatureError::TechnicalError {
                reason: "invalid signature base64",
            });
        };
        let Ok(signature) = Signature::from_der(&signature) else {
            return Err(SignatureError::TechnicalError {
                reason: "invalid signature DER",
            });
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
            return Err(SignatureError::TechnicalError {
                reason: "invalid signed payload",
            });
        };
        if key.verify(&payload, &signature).is_ok() {
            Ok(ValidSignature {
                key_id: key_id.to_string(),
            })
        } else {
            Err(SignatureError::InvalidSignature {
                key_id: key_id.to_string(),
            })
        }
    }

    /// Warns only in Team Logs. A failed log send is reported separately and never filters the message.
    pub fn validate_and_warn<M: Message>(
        &self,
        message: &M,
        logs: &impl TeamLogger,
    ) -> (
        Result<ValidSignature, SignatureError>,
        Option<TeamLogsError>,
    ) {
        let result = self.validate(message);
        let Err(error) = &result else {
            return (result, None);
        };
        let log_error = logs
            .warn(LOGGER_NAME, &warning_message(error, message))
            .err();
        (result, log_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, fs, path::Path};

    #[test]
    fn embedded_keys_match_catalog_and_parse() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("public-keys");
        let filenames: HashSet<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        let keys = embedded_keys().unwrap();
        let ids: HashSet<_> = keys.iter().map(EmbeddedKey::id).collect();
        assert_eq!(keys.len(), 17);
        assert_eq!(ids.len(), keys.len());
        assert!(
            keys.iter()
                .all(|key| matches!(key, EmbeddedKey::Active { .. }))
        );
        assert_eq!(
            filenames,
            ids.iter().map(|id| format!("{id}.pub.b64")).collect()
        );
        let verifier = RecordVerifier::from_embedded_keys().unwrap();
        assert_eq!(verifier.keys.len(), 17);
        assert!(ids.iter().all(|id| verifier.keys.contains_key(*id)));
    }

    #[test]
    fn duplicate_key_id_fails() {
        let mut verifier = RecordVerifier::new();
        let key = embedded_key!("prod-paw-api-inngang-ecdsa-v1");
        verifier.add_key(key.id(), key.encoded()).unwrap();
        assert!(matches!(
            verifier.add_key(key.id(), key.encoded()),
            Err(SigningError::KeyId)
        ));
    }

    #[test]
    fn revoked_key_dates_must_parse_at_startup() {
        let key = (|| -> Result<EmbeddedKey, SigningError> {
            Ok(embedded_key!(
                revoked "prod-paw-api-inngang-ecdsa-v1",
                "2026-10-01T12:30:00Z",
                "2026-10-02T12:30:00Z"
            ))
        })()
        .unwrap();
        assert_eq!(key.id(), "prod-paw-api-inngang-ecdsa-v1");
        assert_eq!(
            key.encoded(),
            embedded_key!("prod-paw-api-inngang-ecdsa-v1").encoded()
        );
        let EmbeddedKey::Revoked {
            revoked_after,
            recorded_at,
            ..
        } = key
        else {
            panic!("expected revoked key");
        };
        assert_eq!(revoked_after.to_rfc3339(), "2026-10-01T12:30:00+00:00");
        assert_eq!(recorded_at.to_rfc3339(), "2026-10-02T12:30:00+00:00");

        assert!(matches!(
            EmbeddedKey::revoked("example", "unused", "invalid", "2026-10-02T12:30:00Z"),
            Err(SigningError::InvalidRevocationDate)
        ));
        assert!(matches!(
            EmbeddedKey::revoked("example", "unused", "2026-10-01T12:30:00Z", "invalid"),
            Err(SigningError::InvalidRevocationDate)
        ));
    }
}
