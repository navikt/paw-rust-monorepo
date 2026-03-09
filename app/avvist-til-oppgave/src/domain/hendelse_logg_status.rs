use thiserror::Error;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, EnumString, Display)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = hendelse_logg_status_not_found,
    parse_err_ty = HendelseLoggStatusParseError
)]
pub enum HendelseLoggStatus {
    OppgaveOpprettet,
    OppgaveIgnorert,
    OppgaveFinnesAllerede,
    EksternOppgaveOpprettelseFeilet,
    EksternOppgaveOpprettet,
    EksternOppgaveFerdigstilt,
    EksternOppgaveFeilregistrert,
}

fn hendelse_logg_status_not_found(status: &str) -> HendelseLoggStatusParseError {
    HendelseLoggStatusParseError::UgyldigStatus(status.to_string())
}

#[derive(Error, Debug, PartialEq)]
pub enum HendelseLoggStatusParseError {
    #[error("Ugyldig HendelseLoggStatus: {0}")]
    UgyldigStatus(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            HendelseLoggStatus::from_str("OPPGAVE_OPPRETTET"),
            Ok(HendelseLoggStatus::OppgaveOpprettet)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("OPPGAVE_FINNES_ALLEREDE"),
            Ok(HendelseLoggStatus::OppgaveFinnesAllerede)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("EKSTERN_OPPGAVE_OPPRETTELSE_FEILET"),
            Ok(HendelseLoggStatus::EksternOppgaveOpprettelseFeilet)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("EKSTERN_OPPGAVE_OPPRETTET"),
            Ok(HendelseLoggStatus::EksternOppgaveOpprettet)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("OPPGAVE_IGNORERT"),
            Ok(HendelseLoggStatus::OppgaveIgnorert)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("EKSTERN_OPPGAVE_FERDIGSTILT"),
            Ok(HendelseLoggStatus::EksternOppgaveFerdigstilt)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("EKSTERN_OPPGAVE_FEILREGISTRERT"),
            Ok(HendelseLoggStatus::EksternOppgaveFeilregistrert)
        );

        let ukjent_status = "UkjentStatus";
        assert!(HendelseLoggStatus::from_str(ukjent_status).is_err());
        assert_eq!(
            HendelseLoggStatus::from_str(ukjent_status).unwrap_err().to_string(),
            format!("Ugyldig HendelseLoggStatus: {}", ukjent_status)
        );
    }
}

