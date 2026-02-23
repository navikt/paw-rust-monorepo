use std::str::FromStr;
use thiserror::Error;
use crate::domain::oppgave_status::OppgaveStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum HendelseLoggStatus {
    OppgaveOpprettet,
    AvvistHendelseMottatt,
    EksternOppgaveOpprettelseFeilet,
    EksternOppgaveOpprettet
}

impl std::fmt::Display for HendelseLoggStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HendelseLoggStatus::OppgaveOpprettet => write!(f, "OppgaveOpprettet"),
            HendelseLoggStatus::AvvistHendelseMottatt => write!(f, "AvvistHendelseMottatt"),
            HendelseLoggStatus::EksternOppgaveOpprettelseFeilet => write!(f, "EksternOppgaveOpprettelseFeilet"),
            HendelseLoggStatus::EksternOppgaveOpprettet => write!(f, "EksternOppgaveOpprettet"),
        }
    }
}

impl FromStr for HendelseLoggStatus {
    type Err = HendelseLoggStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OppgaveOpprettet" => Ok(HendelseLoggStatus::OppgaveOpprettet),
            "AvvistHendelseMottatt" => Ok(HendelseLoggStatus::AvvistHendelseMottatt),
            "EksternOppgaveOpprettelseFeilet" => Ok(HendelseLoggStatus::EksternOppgaveOpprettelseFeilet),
            "EksternOppgaveOpprettet" => Ok(HendelseLoggStatus::EksternOppgaveOpprettet),
            _ => Err(HendelseLoggStatusParseError::UgyldigStatus(s.to_string())),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum HendelseLoggStatusParseError {
    #[error("Ugyldig HendelseLoggStatus: {0}")]
    UgyldigStatus(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_status() {
        assert_eq!(
            HendelseLoggStatus::from_str("OppgaveOpprettet"),
            Ok(HendelseLoggStatus::OppgaveOpprettet)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("AvvistHendelseMottatt"),
            Ok(HendelseLoggStatus::AvvistHendelseMottatt)
        );
        assert_eq!(
            HendelseLoggStatus::from_str("EksternOppgaveOpprettelseFeilet"),
            Ok(HendelseLoggStatus::EksternOppgaveOpprettelseFeilet)
        );

        assert_eq!(
            HendelseLoggStatus::from_str("EksternOppgaveOpprettet"),
            Ok(HendelseLoggStatus::EksternOppgaveOpprettet)
        );

        let ukjent_status = "UkjentStatus";
        assert!(HendelseLoggStatus::from_str(ukjent_status).is_err());
        assert_eq!(
            HendelseLoggStatus::from_str(ukjent_status).unwrap_err().to_string(),
            format!("Ugyldig HendelseLoggStatus: {}", ukjent_status)
        );
    }
}

