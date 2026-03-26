use strum::{Display, EnumIter, EnumString};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, EnumString, Display, EnumIter)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = oppgave_status_not_found,
    parse_err_ty = OppgaveStatusParseError
)]
pub enum OppgaveStatus {
    Ubehandlet,
    Opprettet,
    Ferdigbehandlet,
    Ignorert
}

fn oppgave_status_not_found(status: &str) -> OppgaveStatusParseError {
    OppgaveStatusParseError::UgyldigStatus(status.to_string())
}

#[derive(Error, Debug, PartialEq)]
pub enum OppgaveStatusParseError {
    #[error("Ugyldig oppgavestatus: {0}")]
    UgyldigStatus(String),
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            OppgaveStatus::from_str("UBEHANDLET"),
            Ok(OppgaveStatus::Ubehandlet)
        );
        assert_eq!(
            OppgaveStatus::from_str("OPPRETTET"),
            Ok(OppgaveStatus::Opprettet)
        );
        assert_eq!(
            OppgaveStatus::from_str("FERDIGBEHANDLET"),
            Ok(OppgaveStatus::Ferdigbehandlet)
        );
        assert_eq!(
            OppgaveStatus::from_str("IGNORERT"),
            Ok(OppgaveStatus::Ignorert)
        );

        let ugyldig_status = "UgyldigStatus";
        assert!(OppgaveStatus::from_str(ugyldig_status).is_err());
        assert_eq!(
            OppgaveStatus::from_str(ugyldig_status).unwrap_err().to_string(),
            format!("Ugyldig oppgavestatus: {}", ugyldig_status)
        );
    }
}
