use chrono::{DateTime, Utc};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use p256::pkcs8::DecodePublicKey;

// Keep retired keys for historical verification. The caller decides when each
// key was valid, using topic/partition/offset rather than record timestamp.
// Source: paw-arbeidssoekerregisteret-monorepo-intern and -ekstern,
// lib/kafka-signing/src/main/resources/paw-signing-public-keys/.
pub enum PublicKey {
    Active {
        id: &'static str,
        encoded: VerifyingKey,
    },
    // Revocation is metadata only. Verification still accepts this public key;
    // the caller must decide whether it was allowed for a particular record.
    #[allow(dead_code, reason = "No public keys are revoked yet")]
    Revoked {
        id: &'static str,
        encoded: VerifyingKey,
        revoked_after: DateTime<Utc>,
        recorded_at: DateTime<Utc>,
    },
}

impl PublicKey {
    pub(super) fn id(&self) -> &'static str {
        match self {
            Self::Active { id, .. } | Self::Revoked { id, .. } => id,
        }
    }

    pub(super) fn encoded(&self) -> &VerifyingKey {
        match self {
            Self::Active { encoded, .. } | Self::Revoked { encoded, .. } => encoded,
        }
    }

    pub fn verify(&self, payload: &[u8], signature: &[u8]) -> Result<(), VerificationError> {
        let signature =
            Signature::from_der(signature).map_err(|_| VerificationError::MalformedSignature)?;
        self.encoded()
            .verify(payload, &signature)
            .map_err(|_| VerificationError::SignatureMismatch)
    }

    pub fn active(id: &'static str, encoded: &'static str) -> Result<Self, KeyError> {
        let encoded: VerifyingKey = VerifyingKey::from_public_key_der(encoded.as_bytes())
            .map_err(|_| KeyError::MalformedKey)?;
        Ok(Self::Active { id, encoded })
    }

    #[allow(dead_code, reason = "Used when a key is marked revoked in the catalog")]
    pub fn revoked(
        id: &'static str,
        encoded: &'static str,
        revoked_after: &str,
        recorded_at: &str,
    ) -> Result<Self, KeyError> {
        let parse = |timestamp: &str| {
            DateTime::parse_from_rfc3339(timestamp)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|_| KeyError::InvalidRevocationDateTime {
                    provided_time_as_string: timestamp.to_string(),
                })
        };
        let encoded: VerifyingKey = VerifyingKey::from_public_key_der(encoded.as_bytes())
            .map_err(|_| KeyError::MalformedKey)?;

        Ok(Self::Revoked {
            id,
            encoded,
            revoked_after: parse(revoked_after)?,
            recorded_at: parse(recorded_at)?,
        })
    }
}

macro_rules! active_key {
    ($id:literal) => {
        PublicKey::active(
            $id,
            include_str!(concat!("../../public-keys/", $id, ".pub.b64")),
        )
    };
}

#[allow(unused_macros)]
macro_rules! revoked_key {
    ($id:literal, $revoked_after:literal, $recorded_at:literal) => {
        PublicKey::revoked(
            $id,
            include_str!(concat!("../../public-keys/", $id, ".pub.b64")),
            $revoked_after,
            $recorded_at,
        )
    };
}

pub(super) use active_key;
#[allow(unused_imports)]
pub(super) use revoked_key;

use crate::signing::{KeyError, VerificationError};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::key_catalog::get_embedded_keys;
    use std::{collections::HashSet, fs, path::Path};

    #[test]
    fn embedded_keys_match_catalog_and_parse() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("public-keys");
        let filenames: HashSet<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        let keys = get_embedded_keys();
        let ids: HashSet<_> = keys.iter().map(PublicKey::id).collect();
        assert_eq!(keys.len(), 17);
        assert_eq!(ids.len(), keys.len());
        assert!(
            keys.iter()
                .all(|key| matches!(key, PublicKey::Active { .. }))
        );
        assert_eq!(
            filenames,
            ids.iter().map(|id| format!("{id}.pub.b64")).collect()
        );
    }

    #[test]
    fn revoked_key_dates_must_parse_at_startup() {
        let key = revoked_key!(
            "prod-paw-api-inngang-ecdsa-v1",
            "2026-10-01T12:30:00Z",
            "2026-10-02T12:30:00Z"
        )
        .unwrap();
        assert_eq!(key.id(), "prod-paw-api-inngang-ecdsa-v1");
        let PublicKey::Revoked {
            revoked_after,
            recorded_at,
            ..
        } = key
        else {
            panic!("expected revoked key");
        };
        assert_eq!(revoked_after.to_rfc3339(), "2026-10-01T12:30:00+00:00");
        assert_eq!(recorded_at.to_rfc3339(), "2026-10-02T12:30:00+00:00");

        let invalid_date_time = "invalid".to_string();
        assert!(matches!(
            PublicKey::revoked(
                "example",
                "unused",
                &invalid_date_time,
                "2026-10-02T12:30:00Z"
            ),
            Err(KeyError::InvalidRevocationDateTime {
                ref provided_time_as_string
            }) if provided_time_as_string == &invalid_date_time
        ));
        assert!(matches!(
            PublicKey::revoked("example", "unused", &invalid_date_time, "2026-10-02T12:30:00Z"),
            Err(KeyError::InvalidRevocationDateTime { ref provided_time_as_string })
                if provided_time_as_string == &invalid_date_time
        ));
    }
}
