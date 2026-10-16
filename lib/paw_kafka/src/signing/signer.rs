use std::fs;
use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD};
use p256::SecretKey;
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::DecodePrivateKey;
use rdkafka::{
    message::{Header, Headers, OwnedHeaders, ToBytes},
    producer::FutureRecord,
};

use crate::signing::wire::signature_payload_from_furture_record;
use crate::signing::{KeyError, SignError};

use super::{SIGNATURE_HEADER, SIGNING_KEY_ID_HEADER};

/// Keys are read once at startup; the private key is never logged or embedded in the crate.
pub struct RecordSigner {
    key_id: String,
    key: SigningKey,
}

impl RecordSigner {
    /// Parses a Base64 PKCS#8 DER private key from a mounted secret.
    pub fn from_pkcs8_base64(key_id: impl Into<String>, encoded: &str) -> Result<Self, KeyError> {
        let key_id = key_id.into();
        let der = decode_private_key(encoded)?;
        let secret = SecretKey::from_pkcs8_der(&der).map_err(|_| KeyError::MalformedKey)?;
        Ok(Self {
            key_id,
            key: SigningKey::from(secret),
        })
    }

    /// Reads `PAW_SIGNING_PRIVATE_KEY_PKCS8_BASE64` and `PAW_SIGNING_KEY_ID`
    /// from files in the Nais-mounted secret directory.
    pub fn from_secret_dir(dir: &Path) -> Result<Self, KeyError> {
        Self::from_pkcs8_base64(
            fs::read_to_string(dir.join("PAW_SIGNING_KEY_ID"))
                .map_err(KeyError::IO)?
                .trim(),
            fs::read_to_string(dir.join("PAW_SIGNING_PRIVATE_KEY_PKCS8_BASE64"))
                .map_err(KeyError::IO)?
                .trim(),
        )
    }

    pub fn sign_record<'a, K: ToBytes + ?Sized, P: ToBytes + ?Sized>(
        &self,
        record: FutureRecord<'a, K, P>,
    ) -> Result<FutureRecord<'a, K, P>, SignError> {
        let data_to_sign = signature_payload_from_furture_record(&record)?;
        let signature: Signature = self.key.sign(&data_to_sign);
        let headers = strip_signing_headers(record.headers.as_ref())
            .insert(Header {
                key: SIGNATURE_HEADER,
                value: Some(signature.to_der().as_bytes()),
            })
            .insert(Header {
                key: SIGNING_KEY_ID_HEADER,
                value: Some(self.key_id.as_bytes()),
            });
        Ok(record.headers(headers))
    }
}

fn decode_private_key(input: &str) -> Result<Vec<u8>, KeyError> {
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
    STANDARD.decode(padded).map_err(|_| KeyError::MalformedKey)
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
