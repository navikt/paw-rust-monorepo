use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct HendelseLoggEntry {
    pub status: HendelseLoggStatus,
    pub melding: String,
    pub tidspunkt: DateTime<Utc>,
}

impl HendelseLoggEntry {
    pub fn new(
        status: HendelseLoggStatus,
        melding: String,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            status,
            melding,
            tidspunkt,
        }
    }
}
