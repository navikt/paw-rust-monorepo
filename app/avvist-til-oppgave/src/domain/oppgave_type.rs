use thiserror::Error;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum OppgaveType {
    AvvistUnder18,
}

impl FromStr for OppgaveType {
    type Err = OppgaveTypeParseError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "AvvistUnder18" => Ok(OppgaveType::AvvistUnder18),
            _ => Err(OppgaveTypeParseError::UkjentType(str.to_string())),
        }
    }
}

impl std::fmt::Display for OppgaveType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OppgaveType::AvvistUnder18 => write!(f, "AvvistUnder18"),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum OppgaveTypeParseError {
    #[error("Ukjent oppgavetype: {0}")]
    UkjentType(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            OppgaveType::from_str("AvvistUnder18"),
            Ok(OppgaveType::AvvistUnder18)
        );
        let ukjent_type = "UkjentType";
        assert!(OppgaveType::from_str(ukjent_type).is_err());
        assert_eq!(
            OppgaveType::from_str(ukjent_type).unwrap_err().to_string(),
            format!("Ukjent oppgavetype: {}", ukjent_type)
        );
    }
}
