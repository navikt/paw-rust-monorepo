use crate::signing::key_catalog::get_embedded_keys;
use crate::signing::key_catalog_tools::PublicKey;
use crate::signing::wire::signature_payload_from_message;
use crate::signing::{KeyError, VerificationError};

use super::warning::warning_message;
use super::wire::last_header;
use super::{LOGGER_NAME, SIGNATURE_HEADER, SIGNING_KEY_ID_HEADER};
use base64::{
    Engine,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use p256::ecdsa::{Signature, signature::Verifier};
use paw_team_logs::{TeamLogger, TeamLogsError};
use rdkafka::Message;
use std::collections::HashMap;

/// A signature verified with the public key identified by `key_id`.
/// Whether this key was permitted at the record's offset is up to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidSignature {
    pub key_id: &'static str,
}

#[derive(Default)]
pub struct RecordVerifier {
    keys: HashMap<&'static str, &'static PublicKey>,
}

impl RecordVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads all known public keys, including old keys for historical records.
    /// Fails at startup if any embedded key is invalid.
    pub fn from_embedded_keys() -> Result<Self, KeyError> {
        let mut verifier = Self::new();
        for key in get_embedded_keys() {
            verifier.add_key(key)?;
        }
        Ok(verifier)
    }

    /// Registers an X.509/SPKI DER public key encoded as standard Base64.
    pub fn add_key(&mut self, key: &'static PublicKey) -> Result<(), KeyError> {
        let key_id = key.id();
        if self.keys.contains_key(key_id) {
            return Err(KeyError::DuplicateKeyId { key_id });
        }
        self.keys.insert(key_id, key);
        Ok(())
    }

    /// Examines raw Kafka bytes without modifying or consuming the record.
    /// A valid signature does not imply the key was allowed at this offset.
    pub fn validate<M: Message>(
        &self,
        reference_time: DateTime<Utc>,
        message: &M,
    ) -> Result<ValidSignature, VerificationError> {
        let signature = last_header(message.headers(), SIGNATURE_HEADER);
        let key_id = last_header(message.headers(), SIGNING_KEY_ID_HEADER);
        let Some(signature) = signature else {
            return Err(VerificationError::NoSignaturePresent);
        };
        let Some(key_id) = key_id else {
            return Err(VerificationError::NoKeyIdPresent);
        };
        let Ok(key_id) = std::str::from_utf8(key_id) else {
            return Err(VerificationError::MalformedKeyId);
        };
        let Some(key) = self.keys.get(key_id) else {
            return Err(VerificationError::Key(KeyError::UnknownKeyId {
                key_id: key_id.to_string(),
            }));
        };
        let Ok(encoded_signature) = std::str::from_utf8(signature) else {
            return Err(VerificationError::MalformedSignature);
        };
        let Ok(signature) = URL_SAFE_NO_PAD
            .decode(encoded_signature)
            .or_else(|_| URL_SAFE.decode(encoded_signature))
        else {
            return Err(VerificationError::MalformedSignature);
        };
        let Ok(signature) = Signature::from_der(&signature) else {
            return Err(VerificationError::MalformedSignature);
        };
        let payload = signature_payload_from_message(message)?;
        if key.encoded().verify(&payload, &signature).is_ok() {
            validate_revocation(key, reference_time).map_err(VerificationError::Key)
        } else {
            Err(VerificationError::SignatureMismatch)
        }
    }

    /// Warns only in Team Logs. A failed log send is reported separately and never filters the message.
    pub fn validate_and_warn<M: Message>(
        &self,
        reference_time: DateTime<Utc>,
        message: &M,
        logs: &impl TeamLogger,
    ) -> (
        Result<ValidSignature, VerificationError>,
        Option<TeamLogsError>,
    ) {
        let result = self.validate(reference_time, message);
        let Err(error) = &result else {
            return (result, None);
        };
        let log_error = logs
            .warn(LOGGER_NAME, &warning_message(error, message))
            .err();
        (result, log_error)
    }
}

pub fn validate_revocation(
    key: &PublicKey,
    reference_time: DateTime<Utc>,
) -> Result<ValidSignature, KeyError> {
    match key {
        PublicKey::Active { id, encoded: _ } => Ok(ValidSignature { key_id: id }),
        PublicKey::Revoked {
            id,
            encoded: _,
            revoked_after,
            recorded_at,
        } => {
            if reference_time >= *revoked_after {
                Err(KeyError::KeyRevoked {
                    key_id: id,
                    not_valid_after: *revoked_after,
                    recorded_revocation: *recorded_at,
                    reference_time,
                })
            } else {
                Ok(ValidSignature { key_id: id })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_key_id_fails() {
        let mut verifier = RecordVerifier::new();
        let key = get_embedded_keys()
            .iter()
            .find(|key| key.id() == "prod-paw-api-inngang-ecdsa-v1")
            .unwrap();
        verifier.add_key(key).unwrap();
        assert!(matches!(
            verifier.add_key(key),
            Err(KeyError::DuplicateKeyId {
                key_id: "prod-paw-api-inngang-ecdsa-v1"
            })
        ));
    }

    #[test]
    fn embedded_keys_load_into_verifier() {
        let keys = get_embedded_keys();
        let verifier = RecordVerifier::from_embedded_keys().unwrap();
        assert_eq!(verifier.keys.len(), 17);
        assert!(keys.iter().all(|key| verifier.keys.contains_key(key.id())));
    }
}
