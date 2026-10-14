use super::SigningError;
use chrono::{DateTime, Utc};

// Keep retired keys for historical verification. The caller decides when each
// key was valid, using topic/partition/offset rather than record timestamp.
// Source: paw-arbeidssoekerregisteret-monorepo-intern and -ekstern,
// lib/kafka-signing/src/main/resources/paw-signing-public-keys/.
pub(super) enum EmbeddedKey {
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
    pub(super) fn id(&self) -> &'static str {
        match self {
            Self::Active { id, .. } | Self::Revoked { id, .. } => id,
        }
    }

    pub(super) fn encoded(&self) -> &'static str {
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

/// The active form is `embedded_key!("prod-paw-api-inngang-ecdsa-v1")`.
///
/// ```ignore
/// embedded_key!(revoked "prod-paw-api-inngang-ecdsa-v1",
///     "2026-10-01T12:30:00Z", "2026-10-02T12:30:00Z")
/// ```
///
/// `revoked_after` and `recorded_at` are stored as `DateTime<Utc>`. `Revoked`
/// is metadata only and is not enforced by verification. Retain historical keys.
pub(super) fn embedded_keys() -> Result<Vec<EmbeddedKey>, SigningError> {
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
