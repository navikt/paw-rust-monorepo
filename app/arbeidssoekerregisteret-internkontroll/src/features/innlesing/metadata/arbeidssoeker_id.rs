use std::fmt;

/// Registerets interne id for en arbeidssøker. Egen type så den ikke kan
/// forveksles med andre tall, som xid, offset eller tidsstempler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct ArbeidssoekerId(i64);

impl ArbeidssoekerId {
    pub const fn new(verdi: i64) -> Self {
        Self(verdi)
    }
}

impl fmt::Display for ArbeidssoekerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
