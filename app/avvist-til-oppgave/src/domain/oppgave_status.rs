use thiserror::Error;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum OppgaveStatus {
    Ubehandlet,
    Ferdigbehandlet,
}

impl FromStr for OppgaveStatus {
    type Err = OppgaveStatusParseError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "Ubehandlet" => Ok(OppgaveStatus::Ubehandlet),
            "Ferdigbehandlet" => Ok(OppgaveStatus::Ferdigbehandlet),
            _ => Err(OppgaveStatusParseError::UkjentStatus(str.to_string())),
        }
    }
}

impl std::fmt::Display for OppgaveStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OppgaveStatus::Ubehandlet => write!(f, "Ubehandlet"),
            OppgaveStatus::Ferdigbehandlet => write!(f, "Ferdigbehandlet"),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum OppgaveStatusParseError {
    #[error("Ukjent oppgavestatus: {0}")]
    UkjentStatus(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            OppgaveStatus::from_str("Ubehandlet"),
            Ok(OppgaveStatus::Ubehandlet)
        );
        assert_eq!(
            OppgaveStatus::from_str("Ferdigbehandlet"),
            Ok(OppgaveStatus::Ferdigbehandlet)
        );

        let ukjent_status = "UkjentStatus";
        assert!(OppgaveStatus::from_str(ukjent_status).is_err());
        assert_eq!(
            OppgaveStatus::from_str(ukjent_status).unwrap_err().to_string(),
            format!("Ukjent oppgavestatus: {}", ukjent_status)
        );
    }
}
