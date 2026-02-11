use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum HendelseLoggStatus {
    OppgaveOpprettet,
    AvvistHendelseMottatt,
}

impl HendelseLoggStatus {
    pub(crate) fn standard_melding(&self) -> String {
        match self {
            HendelseLoggStatus::OppgaveOpprettet => "Opprettet oppgave".to_string(),
            HendelseLoggStatus::AvvistHendelseMottatt => "Avvist hendelse mottatt".to_string(),
        }
    }
}

impl fmt::Display for HendelseLoggStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
            _ => Err(HendelseLoggStatusParseError {
                message: format!("Ugyldig HendelseLoggStatus: {}", s),
            }),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct HendelseLoggStatusParseError {
    pub(crate) message: String,
}

impl fmt::Display for HendelseLoggStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HendelseLoggStatusParseError {}
