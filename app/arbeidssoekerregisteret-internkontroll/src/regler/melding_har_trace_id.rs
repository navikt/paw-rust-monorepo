use crate::{
    database::record_derived::RecordDerived,
    regler::{
        alvorlighet::{Alvorlighet, Utfall},
        funn::Funn,
        nokkel::{Nivaa, Nokkel},
        regel_katalog::RegelId,
        regler_task::Regel,
    },
};

const REGEL_VERSJON: i32 = 1;
const NIVAA: Nivaa = Nivaa::Melding;
const MANGLER_TRACE_ID: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Info,
};

pub fn regel_melding_har_trace_id() -> Regel {
    Regel::new(RegelId::MeldingHarTraceId, REGEL_VERSJON, NIVAA)
}

/// Each record is checked alone, so no data is read beyond the record itself.
/// A record is never written twice, so OK needs no stored state to delete;
/// AltOk is still returned to keep the contract uniform.
pub fn melding_har_trace_id(xid: i64, record: &RecordDerived) -> Funn {
    let nokkel = Nokkel::Melding(xid);
    if record.trace_id.is_some() {
        Funn::AltOk(nokkel)
    } else {
        Funn::Feil(nokkel, MANGLER_TRACE_ID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{payload::PayloadStatus, signature::Signature};
    use chrono::DateTime;
    use uuid::Uuid;

    fn record(trace_id: Option<Uuid>) -> RecordDerived {
        RecordDerived {
            topic_id: 1,
            record_partition: 0,
            record_offset: 0,
            signature: Signature::Missing,
            timestamp_delta_ms: None,
            trace_id,
            payload_status: PayloadStatus::Ok,
            meldingstype: None,
            periode_id: None,
            arbeidssoeker_id: None,
            derived_at: DateTime::from_timestamp_millis(10_000).unwrap(),
        }
    }

    #[test]
    fn med_trace_id_er_ok() {
        assert_eq!(
            melding_har_trace_id(7, &record(Some(Uuid::from_u128(1)))),
            Funn::AltOk(Nokkel::Melding(7))
        );
    }

    #[test]
    fn uten_trace_id_gir_info_funn() {
        assert_eq!(
            melding_har_trace_id(7, &record(None)),
            Funn::Feil(Nokkel::Melding(7), MANGLER_TRACE_ID)
        );
    }
}
