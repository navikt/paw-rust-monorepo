use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum OppgaveType {
    AvvistUnder18,
}

impl FromStr for OppgaveType {
    type Err = OppgaveTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AvvistUnder18" => Ok(OppgaveType::AvvistUnder18),
            _ => Err(OppgaveTypeParseError {
                message: format!("Ukjent oppgavetype: {}", s),
            }),
        }
    }
}

impl fmt::Display for OppgaveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OppgaveType::AvvistUnder18 => write!(f, "AvvistUnder18"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct OppgaveTypeParseError {
    message: String,
}

impl fmt::Display for OppgaveTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OppgaveTypeParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            OppgaveType::from_str("AvvistUnder18"),
            Ok(OppgaveType::AvvistUnder18)
        );
        assert!(OppgaveType::from_str("UkjentType").is_err());
        assert_eq!(
            OppgaveType::from_str("UkjentType")
                .unwrap_err()
                .to_string(),
            "Ukjent oppgavetype: UkjentType"
        );
    }
}