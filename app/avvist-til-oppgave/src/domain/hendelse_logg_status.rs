use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum HendelseLoggStatus {
    OppgaveOpprettet,
    AvvistHendelseMottatt,
}

impl std::fmt::Display for HendelseLoggStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HendelseLoggStatus::OppgaveOpprettet => write!(f, "OppgaveOpprettet"),
            HendelseLoggStatus::AvvistHendelseMottatt => write!(f, "AvvistHendelseMottatt"),
        }
    }
}

impl FromStr for HendelseLoggStatus {
    type Err = HendelseLoggStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OppgaveOpprettet" => Ok(HendelseLoggStatus::OppgaveOpprettet),
            "AvvistHendelseMottatt" => Ok(HendelseLoggStatus::AvvistHendelseMottatt),
            _ => Err(HendelseLoggStatusParseError::UgyldigStatus(s.to_string())),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum HendelseLoggStatusParseError {
    #[error("Ugyldig HendelseLoggStatus: {0}")]
    UgyldigStatus(String),
}
