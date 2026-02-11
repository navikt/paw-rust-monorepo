use crate::domain::hendelse_logg_status::{HendelseLoggStatus, HendelseLoggStatusParseError};
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct HendelseLoggEntry {
    pub status: HendelseLoggStatus,
    pub tidspunkt: DateTime<Utc>,
}

impl HendelseLoggEntry {
    pub fn new(status: String, tidspunkt: DateTime<Utc>) -> Result<Self, HendelseLoggEntryError> {
        Ok(Self {
            status: status.parse()?,
            tidspunkt,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct HendelseLoggEntryError {
    message: String,
}

impl fmt::Display for HendelseLoggEntryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HendelseLoggEntryError {}

impl From<HendelseLoggStatusParseError> for HendelseLoggEntryError {
    fn from(err: HendelseLoggStatusParseError) -> Self {
        HendelseLoggEntryError {
            message: err.message,
        }
    }
}

#[test]
fn test_new_with_invalid_status() {
    let tidspunkt = Utc::now();
    let ugyldig_logg_status = "UgyldigStatus";
    let result = HendelseLoggEntry::new(ugyldig_logg_status.to_string(), tidspunkt);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("Ugyldig HendelseLoggStatus: {}", ugyldig_logg_status)
    );
}
