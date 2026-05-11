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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let tidspunkt = Utc::now();
        let entry = HendelseLoggEntry::new(
            HendelseLoggStatus::OppgaveOpprettet,
            "test melding".to_string(),
            tidspunkt,
        );

        assert_eq!(entry.status, HendelseLoggStatus::OppgaveOpprettet);
        assert_eq!(entry.melding, "test melding");
        assert_eq!(entry.tidspunkt, tidspunkt);
    }
}
