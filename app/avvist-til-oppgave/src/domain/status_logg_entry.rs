use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct StatusLoggEntry {
    pub status: String, //TODO maskinlesbar enum?
    pub tidspunkt: DateTime<Utc>,
}

impl StatusLoggEntry {
    pub fn new(status: String, tidspunkt: DateTime<Utc>) -> Self {
        Self { status, tidspunkt }
    }
}
