use crate::domain::hendelse_logg_status::{HendelseLoggStatus, HendelseLoggStatusParseError};
use chrono::{DateTime, Utc};
use thiserror::Error;

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

#[derive(Error, Debug, PartialEq)]
pub enum HendelseLoggEntryError {
    #[error(transparent)]
    ParseError(#[from] HendelseLoggStatusParseError),
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
