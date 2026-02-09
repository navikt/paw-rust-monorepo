#[derive(Debug, PartialEq)]
pub enum OppgaveType {
    AvvistUnder18
}

impl OppgaveType {
    pub fn to_string(&self) -> String {
        match self {
            OppgaveType::AvvistUnder18 => "AvvistUnder18".to_string(),
        }
    }
    pub fn from_str(oppgave_type: String) -> Option<Self> {
        match oppgave_type.as_str() {
            "AvvistUnder18" => Some(OppgaveType::AvvistUnder18),
            _ => None,
        }
    }
}