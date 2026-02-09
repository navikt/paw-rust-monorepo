#[derive(Debug, Clone, PartialEq)]
pub enum OppgaveStatus {
    Ubehandlet,
    Feilet,
    Ferdigbehandlet,
}

impl OppgaveStatus {
    pub fn to_string(&self) -> String {
        match self {
            OppgaveStatus::Ubehandlet => "Ubehandlet".to_string(),
            OppgaveStatus::Feilet => "Feilet".to_string(),
            OppgaveStatus::Ferdigbehandlet => "Ferdigbehandlet".to_string(),
        }
    }
    pub fn from_str(status: String) -> Option<Self> {
        match status.as_str() {
            "Ubehandlet" => Some(OppgaveStatus::Ubehandlet),
            "Feilet" => Some(OppgaveStatus::Feilet),
            "Ferdigbehandlet" => Some(OppgaveStatus::Ferdigbehandlet),
            _ => None,
        }
    }
}
