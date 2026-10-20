//! Kotlin-compatible P-256/SHA-256 Kafka record signing and observational validation.
//! Validation never filters Kafka records. Warnings go only to private Team Logs.

use chrono::{DateTime, Utc};
use thiserror::Error;

mod key_catalog;
mod key_catalog_tools;
mod signer;
mod verifier;
mod warning;
mod wire;

pub use key_catalog_tools::PublicKey;
pub use signer::{RecordSigner, strip_signing_headers};
pub use verifier::{RecordVerifier, ValidSignature};

pub const SIGNATURE_HEADER: &str = "x-paw-signature";
pub const SIGNING_KEY_ID_HEADER: &str = "x-paw-signing-key-id";
const TRACEPARENT_HEADER: &str = "traceparent";
const LOGGER_NAME: &str = "team-logs-logger";

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("Malformed key")]
    MalformedKey,
    #[error("unknown key id: {key_id}")]
    UnknownKeyId { key_id: String },
    #[error(
        "signing key no longer valid after {not_valid_after}, revocation recorded at {recorded_revocation}, reference time {reference_time}"
    )]
    KeyRevoked {
        key_id: &'static str,
        not_valid_after: DateTime<Utc>,
        recorded_revocation: DateTime<Utc>,
        reference_time: DateTime<Utc>,
    },
    #[error("invalid revocation date time: {provided_time_as_string}")]
    InvalidRevocationDateTime { provided_time_as_string: String },
    #[error("failed to read key from storage")]
    IO(#[from] std::io::Error),
    #[error("duplicate key id: {key_id}")]
    DuplicateKeyId { key_id: &'static str },
}

#[derive(Debug, Error)]
pub enum PayloadError {
    #[error("to many bytes")]
    ToManyBytes,
    #[error("payload timestamp is missing")]
    TimestampMissing,
}

#[derive(Debug, Error)]
pub enum SignError {
    #[error("payload error: {0}")]
    Payload(#[from] PayloadError),
}

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("signature is malformed (key {key_id})")]
    MalformedSignature { key_id: &'static str },
    #[error("signature verification failed: cryptographic mismatch (key {key_id})")]
    SignatureMismatch { key_id: &'static str },
    #[error("signature header is missing")]
    NoSignaturePresent,
    #[error("signing key id header is missing")]
    NoKeyIdPresent,
    #[error("key error: {0}")]
    Key(#[from] KeyError),
    #[error("key id is malformed")]
    MalformedKeyId,
    #[error("payload error for key {key_id}: {error}")]
    Payload {
        key_id: &'static str,
        #[source]
        error: PayloadError,
    },
}
