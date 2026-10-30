use uuid::Uuid;

use crate::arbeidssoeker_id::ArbeidssoekerId;
use crate::database::{record_derived::RecordDerived, xid8::Xid8};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nivaa {
    Melding,
    Trace,
    Periode,
    Arbeidssoeker,
}

impl Nivaa {
    pub const ALLE: [Nivaa; 4] = [
        Self::Melding,
        Self::Trace,
        Self::Periode,
        Self::Arbeidssoeker,
    ];

    pub fn fra_str(s: &str) -> Option<Self> {
        Self::ALLE.into_iter().find(|n| n.as_str() == s)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Melding => "melding",
            Self::Trace => "trace",
            Self::Periode => "periode",
            Self::Arbeidssoeker => "arbeidssoeker",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nokkel {
    Melding(Xid8),
    Trace(Uuid),
    Periode(Uuid),
    Arbeidssoeker(ArbeidssoekerId),
}

impl Nokkel {
    pub fn nivaa(&self) -> Nivaa {
        match self {
            Self::Melding(_) => Nivaa::Melding,
            Self::Trace(_) => Nivaa::Trace,
            Self::Periode(_) => Nivaa::Periode,
            Self::Arbeidssoeker(_) => Nivaa::Arbeidssoeker,
        }
    }

    pub fn fra_record(xid: Xid8, record: &RecordDerived, nivaa: Nivaa) -> Option<Self> {
        match nivaa {
            Nivaa::Melding => Some(Self::Melding(xid)),
            Nivaa::Trace => record.trace_id.map(Self::Trace),
            Nivaa::Periode => record.periode_id.map(Self::Periode),
            Nivaa::Arbeidssoeker => record.arbeidssoeker_id.map(Self::Arbeidssoeker),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    use crate::database::{payload::PayloadStatus, signature::Signature};

    fn record() -> RecordDerived {
        RecordDerived {
            topic_id: 1,
            record_partition: 2,
            record_offset: 3,
            signature: Signature::Missing,
            timestamp_delta_ms: None,
            trace_id: Some(Uuid::from_u128(10)),
            payload_status: PayloadStatus::Ok,
            meldingstype: None,
            periode_id: Some(Uuid::from_u128(20)),
            arbeidssoeker_id: Some(ArbeidssoekerId::new(30)),
            derived_at: DateTime::<Utc>::UNIX_EPOCH,
        }
    }

    #[test]
    fn bygger_nokkel_fra_record_paa_alle_nivaaer() {
        let record = record();
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Melding),
            Some(Nokkel::Melding(Xid8::new(5)))
        );
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Trace),
            Some(Nokkel::Trace(Uuid::from_u128(10)))
        );
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Periode),
            Some(Nokkel::Periode(Uuid::from_u128(20)))
        );
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Arbeidssoeker),
            Some(Nokkel::Arbeidssoeker(ArbeidssoekerId::new(30)))
        );
    }

    #[test]
    fn manglende_nivaa_id_gir_ingen_nokkel() {
        let record = RecordDerived {
            trace_id: None,
            periode_id: None,
            arbeidssoeker_id: None,
            ..record()
        };
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Trace),
            None
        );
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Periode),
            None
        );
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Arbeidssoeker),
            None
        );
        assert_eq!(
            Nokkel::fra_record(Xid8::new(5), &record, Nivaa::Melding),
            Some(Nokkel::Melding(Xid8::new(5)))
        );
    }

    #[test]
    fn nivaa_er_tilgjengelig_som_streng() {
        assert_eq!(Nivaa::Melding.as_str(), "melding");
        assert_eq!(Nivaa::Trace.as_str(), "trace");
        assert_eq!(Nivaa::Periode.as_str(), "periode");
        assert_eq!(Nivaa::Arbeidssoeker.as_str(), "arbeidssoeker");
        assert_eq!(Nokkel::Periode(Uuid::nil()).nivaa(), Nivaa::Periode);
    }
}
