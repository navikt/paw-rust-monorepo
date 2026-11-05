#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nivaa {
    Melding,
    Trace,
    Periode,
    Arbeidssoeker,
}

impl Nivaa {
    pub const ALLE: [Nivaa; 4] = [
        Self::Melding,
        Self::Trace,
        Self::Periode,
        Self::Arbeidssoeker,
    ];

    pub fn fra_str(s: &str) -> Option<Self> {
        Self::ALLE.into_iter().find(|n| n.as_str() == s)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Melding => "melding",
            Self::Trace => "trace",
            Self::Periode => "periode",
            Self::Arbeidssoeker => "arbeidssoeker",
        }
    }
}
