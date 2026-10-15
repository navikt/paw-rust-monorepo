use std::sync::LazyLock;

use crate::signing::{
    KeyError,
    key_catalog_tools::{PublicKey, active_key},
};

/// The active form is `active_key!("prod-paw-api-inngang-ecdsa-v1")`.
///
/// ```ignore
/// active_key!(revoked "prod-paw-api-inngang-ecdsa-v1",
///     "2026-10-01T12:30:00Z", "2026-10-02T12:30:00Z")
/// ```
///
/// `revoked_after` and `recorded_at` are stored as `DateTime<Utc>`. `Revoked`
/// is metadata only and is not enforced by verification. Retain historical keys.
fn embedded_keys() -> Result<Vec<PublicKey>, KeyError> {
    Ok(vec![
        active_key!("dev-paw-api-bekreftelse-ecdsa-v1")?,
        active_key!("dev-paw-bekreftelse-filter-ecdsa-v1")?,
        active_key!("dev-paw-bekreftelse-tjeneste-ecdsa-v1")?,
        active_key!("dev-paw-bekreftelse-utgang-ecdsa-v1")?,
        active_key!("dev-paw-egenvurdering-api-ecdsa-v1")?,
        active_key!("dev-paw-event-processor-ecdsa-v1")?,
        active_key!("dev-paw-profilering-ecdsa-v1")?,
        active_key!("paw-api-inngang-kafka-signing-key")?,
        active_key!("paw-api-inngang-kafka-signing-key-v2")?,
        active_key!("prod-paw-api-bekreftelse-ecdsa-v1")?,
        active_key!("prod-paw-api-inngang-ecdsa-v1")?,
        active_key!("prod-paw-bekreftelse-filter-ecdsa-v1")?,
        active_key!("prod-paw-bekreftelse-tjeneste-ecdsa-v1")?,
        active_key!("prod-paw-bekreftelse-utgang-ecdsa-v1")?,
        active_key!("prod-paw-egenvurdering-api-ecdsa-v1")?,
        active_key!("prod-paw-event-processor-ecdsa-v1")?,
        active_key!("prod-paw-profilering-ecdsa-v1")?,
    ])
}

static EMMBEDDED_KEYS: LazyLock<Vec<PublicKey>> =
    LazyLock::new(|| embedded_keys().expect("Embedded keys must be valid"));

pub(super) fn get_embedded_keys() -> &'static [PublicKey] {
    EMMBEDDED_KEYS.as_slice()
}
