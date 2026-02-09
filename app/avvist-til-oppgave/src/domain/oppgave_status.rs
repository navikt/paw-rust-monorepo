use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum OppgaveStatus {
    Ubehandlet,
    Ferdigbehandlet,
}

impl FromStr for OppgaveStatus {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "Ubehandlet" => Ok(OppgaveStatus::Ubehandlet),
            "Ferdigbehandlet" => Ok(OppgaveStatus::Ferdigbehandlet),
            _ => Err(()),
        }
    }
}

impl fmt::Display for OppgaveStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OppgaveStatus::Ubehandlet => write!(f, "Ubehandlet"),
            OppgaveStatus::Ferdigbehandlet => write!(f, "Ferdigbehandlet"),
        }
    }
}
