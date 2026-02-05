#[derive(Debug)]
pub enum AvvistMeldingStatus {
    Ubehandlet,
    Feilet,
    Ferdigbehandlet,
}

impl AvvistMeldingStatus {
    pub fn to_str(&self) -> &'static str {
        match self {
            AvvistMeldingStatus::Ubehandlet => "Ubehandlet",
            AvvistMeldingStatus::Feilet => "Feilet",
            AvvistMeldingStatus::Ferdigbehandlet => "Ferdigbehandlet",
        }
    }
    pub fn from_str(status: &str) -> Option<Self> {
        match status {
            "Ubehandlet" => Some(AvvistMeldingStatus::Ubehandlet),
            "Feilet" => Some(AvvistMeldingStatus::Feilet),
            "Ferdigbehandlet" => Some(AvvistMeldingStatus::Ferdigbehandlet),
            _ => None,
        }
    }
}
