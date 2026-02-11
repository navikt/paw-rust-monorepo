use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct HendelseLoggEntry {
    pub status: String, //TODO maskinlesbar enum?
    pub tidspunkt: DateTime<Utc>,
}

impl HendelseLoggEntry {
    pub fn new(status: String, tidspunkt: DateTime<Utc>) -> Self {
        Self { status, tidspunkt }
    }
}
