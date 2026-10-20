/// Mirrors the Postgres enum `signature_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "signature_status", rename_all = "snake_case")]
pub enum SignatureStatus {
    Valid,
    SignatureMismatch,
    MalformedSignature,
    KeyRevoked,
    PayloadTooLarge,
    MissingSignature,
    UnknownKey,
    NoRecordTimestamp,
    TechnicalError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signature {
    Valid { signing_key_id: String },
    CryptographicMismatch { signing_key_id: String },
    Malformed { signing_key_id: String },
    KeyRevoked { signing_key_id: String },
    PayloadTooLarge { signing_key_id: String },
    Missing,
    UnknownKey,
    NoRecordTimestamp,
    TechnicalError,
}

impl Signature {
    pub fn status(&self) -> SignatureStatus {
        match self {
            Signature::Valid { .. } => SignatureStatus::Valid,
            Signature::CryptographicMismatch { .. } => SignatureStatus::SignatureMismatch,
            Signature::Malformed { .. } => SignatureStatus::MalformedSignature,
            Signature::KeyRevoked { .. } => SignatureStatus::KeyRevoked,
            Signature::PayloadTooLarge { .. } => SignatureStatus::PayloadTooLarge,
            Signature::Missing => SignatureStatus::MissingSignature,
            Signature::UnknownKey => SignatureStatus::UnknownKey,
            Signature::NoRecordTimestamp => SignatureStatus::NoRecordTimestamp,
            Signature::TechnicalError => SignatureStatus::TechnicalError,
        }
    }

    pub fn signing_key_id(&self) -> Option<&str> {
        match self {
            Signature::Valid { signing_key_id }
            | Signature::CryptographicMismatch { signing_key_id }
            | Signature::Malformed { signing_key_id }
            | Signature::KeyRevoked { signing_key_id }
            | Signature::PayloadTooLarge { signing_key_id } => Some(signing_key_id),
            Signature::Missing
            | Signature::UnknownKey
            | Signature::NoRecordTimestamp
            | Signature::TechnicalError => None,
        }
    }

    /// Fails on combinations the table's CHECK constraint does not allow.
    pub fn from_columns(
        status: SignatureStatus,
        signing_key_id: Option<String>,
    ) -> Result<Self, InvalidSignatureColumns> {
        let invalid = |signing_key_id| InvalidSignatureColumns {
            status,
            signing_key_id,
        };
        let with_key = |make: fn(String) -> Signature, id: Option<String>| {
            id.map(make).ok_or_else(|| invalid(None))
        };
        let without_key = |signature: Signature, id: Option<String>| match id {
            None => Ok(signature),
            Some(id) => Err(invalid(Some(id))),
        };
        match status {
            SignatureStatus::Valid => with_key(
                |signing_key_id| Signature::Valid { signing_key_id },
                signing_key_id,
            ),
            SignatureStatus::SignatureMismatch => with_key(
                |signing_key_id| Signature::CryptographicMismatch { signing_key_id },
                signing_key_id,
            ),
            SignatureStatus::MalformedSignature => with_key(
                |signing_key_id| Signature::Malformed { signing_key_id },
                signing_key_id,
            ),
            SignatureStatus::KeyRevoked => with_key(
                |signing_key_id| Signature::KeyRevoked { signing_key_id },
                signing_key_id,
            ),
            SignatureStatus::PayloadTooLarge => with_key(
                |signing_key_id| Signature::PayloadTooLarge { signing_key_id },
                signing_key_id,
            ),
            SignatureStatus::MissingSignature => without_key(Signature::Missing, signing_key_id),
            SignatureStatus::UnknownKey => without_key(Signature::UnknownKey, signing_key_id),
            SignatureStatus::NoRecordTimestamp => {
                without_key(Signature::NoRecordTimestamp, signing_key_id)
            }
            SignatureStatus::TechnicalError => {
                without_key(Signature::TechnicalError, signing_key_id)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid signature columns: status {status:?} with signing_key_id {signing_key_id:?}")]
pub struct InvalidSignatureColumns {
    pub status: SignatureStatus,
    pub signing_key_id: Option<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn alle_signaturer() -> Vec<Signature> {
        let key = || "key-a".to_string();
        vec![
            Signature::Valid {
                signing_key_id: key(),
            },
            Signature::CryptographicMismatch {
                signing_key_id: key(),
            },
            Signature::Malformed {
                signing_key_id: key(),
            },
            Signature::KeyRevoked {
                signing_key_id: key(),
            },
            Signature::PayloadTooLarge {
                signing_key_id: key(),
            },
            Signature::Missing,
            Signature::UnknownKey,
            Signature::NoRecordTimestamp,
            Signature::TechnicalError,
        ]
    }

    #[test]
    fn signatur_konverteres_begge_veier() {
        for signature in alle_signaturer() {
            assert_eq!(
                Signature::from_columns(
                    signature.status(),
                    signature.signing_key_id().map(str::to_owned),
                ),
                Ok(signature)
            );
        }
    }

    #[test]
    fn ugyldige_kombinasjoner_avvises() {
        for signature in alle_signaturer() {
            let wrong_key = match signature.signing_key_id() {
                Some(_) => None,
                None => Some("key-a".to_string()),
            };
            assert!(
                Signature::from_columns(signature.status(), wrong_key).is_err(),
                "{:?} skal avvises med feil nøkkel-ID",
                signature.status()
            );
        }
    }
}
