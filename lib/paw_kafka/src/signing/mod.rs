//! Kotlin-compatible P-256/SHA-256 Kafka record signing and observational validation.
//! Validation never filters Kafka records. Warnings go only to private Team Logs.

use thiserror::Error;

mod signer;
mod verifier;
mod warning;
mod wire;

pub use signer::{RecordSigner, strip_signing_headers};
pub use verifier::{RecordVerifier, SignatureError, ValidSignature};
pub use wire::signature_payload;

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
