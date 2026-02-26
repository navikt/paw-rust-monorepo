use strum::{Display, EnumString};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, EnumString, Display)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = oppgave_type_not_found,
    parse_err_ty = OppgaveTypeParseError
)]
pub enum OppgaveType {
    #[strum(serialize = "AVVIST_UNDER_18")]
    AvvistUnder18,
}

fn oppgave_type_not_found(type_: &str) -> OppgaveTypeParseError {
    OppgaveTypeParseError::UkjentType(type_.to_string())
}

#[derive(Error, Debug, PartialEq)]
pub enum OppgaveTypeParseError {
    #[error("Ukjent oppgavetype: {0}")]
    UkjentType(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            OppgaveType::from_str("AVVIST_UNDER_18"),
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
