#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArbeidssoekerperiodeId(pub uuid::Uuid);

impl From<uuid::Uuid> for ArbeidssoekerperiodeId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
    }
}
