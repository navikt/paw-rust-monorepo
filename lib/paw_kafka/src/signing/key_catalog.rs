use crate::signing::{
    SigningError,
    key_catalog_tools::{EmbeddedKey, embedded_key},
};

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
