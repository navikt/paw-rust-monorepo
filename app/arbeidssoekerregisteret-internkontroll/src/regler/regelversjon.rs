use std::fmt;

/// Versjon av en regel. Økes når regelens logikk eller utfall endres, og
/// gjør at regelen kjøres på nytt fra start med egne funn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct Regelversjon(i32);

impl Regelversjon {
    pub const fn new(verdi: i32) -> Self {
        Self(verdi)
    }
}

impl fmt::Display for Regelversjon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
