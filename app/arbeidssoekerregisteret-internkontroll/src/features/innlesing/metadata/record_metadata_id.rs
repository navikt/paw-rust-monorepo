use std::fmt;

/// Primærnøkkelen i `record_metadata`. Egen type så den ikke kan forveksles
/// med andre tall, som offset, arbeidssøker-id eller tidsstempler.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type, serde::Deserialize,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct RecordMetadataId(i64);

impl RecordMetadataId {
    pub const fn new(verdi: i64) -> Self {
        Self(verdi)
    }
}

impl fmt::Display for RecordMetadataId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
