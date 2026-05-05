use fmt::Display;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArbeidssøkerId(pub i64);

impl From<i64> for ArbeidssøkerId {
    fn from(id: i64) -> Self {
        ArbeidssøkerId(id)
    }
}

impl From<ArbeidssøkerId> for i64 {
    fn from(id: ArbeidssøkerId) -> Self {
        id.0
    }
}

impl Display for ArbeidssøkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
