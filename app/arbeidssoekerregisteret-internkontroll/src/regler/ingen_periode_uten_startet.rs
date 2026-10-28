use sqlx::PgConnection;

use crate::{
    database::{
        payload::Meldingstype,
        record_derived::{RecordDerived, get_at_xid_including_preceding_trace_records},
    },
    regler::{funn::Funn, regel_katalog::RegelId, regler_task::Regel},
};

const REGEL_VERSJON: i32 = 1;
const PERIODE_STARTET_UTEN_HENDELSE_STARTET: i16 = 1;

pub fn regel_ingen_periode_uten_startet() -> Regel {
    Regel::new(RegelId::IngenPeriodeUtenStartet, REGEL_VERSJON)
}

fn alt_ok(trace_id: uuid::Uuid) -> Funn {
    Funn::AltOk {
        regel_id: RegelId::IngenPeriodeUtenStartet,
        regel_versjon: REGEL_VERSJON,
        trace_id,
    }
}

fn feil(trace_id: uuid::Uuid) -> Funn {
    Funn::Feil(crate::regler::funn::Detaljer {
        regel_id: RegelId::IngenPeriodeUtenStartet,
        regel_versjon: REGEL_VERSJON,
        trace_id,
        utfall: PERIODE_STARTET_UTEN_HENDELSE_STARTET,
    })
}

pub async fn ingen_periode_uten_startet(
    conn: &mut PgConnection,
    xid: i64,
) -> Result<Funn, sqlx::Error> {
    let data = get_at_xid_including_preceding_trace_records(conn, xid).await?;
    Ok(evaluate_ingen_periode_uten_startet(data))
}

/// `data` is the trace up to and including the record being evaluated, in xid
/// order, so the evaluated record is the last one.
///
/// - `PeriodeStartet` is OK only if a `HendelseStartet` came earlier in the trace.
/// - `HendelseStartet` after a `PeriodeStartet` in the trace is OK: this AltOk
///   replaces the Feil written when the `PeriodeStartet` was read first.
///   Without a `PeriodeStartet` (for example a duplicate) it is not relevant.
/// - Every other message type is not relevant.
pub fn evaluate_ingen_periode_uten_startet(data: Vec<RecordDerived>) -> Funn {
    let Some(gjeldende) = data.last() else {
        return Funn::IngenTraceId;
    };
    let Some(trace_id) = gjeldende.trace_id else {
        return Funn::IngenTraceId;
    };
    match gjeldende.meldingstype {
        Some(Meldingstype::HendelseStartet) => {
            if finnes(&data, Meldingstype::PeriodeStartet) {
                alt_ok(trace_id)
            } else {
                Funn::IkkeAktuell
            }
        }
        Some(Meldingstype::PeriodeStartet) => {
            if finnes(&data, Meldingstype::HendelseStartet) {
                alt_ok(trace_id)
            } else {
                feil(trace_id)
            }
        }
        _ => Funn::IkkeAktuell,
    }
}

fn finnes(data: &[RecordDerived], meldingstype: Meldingstype) -> bool {
    data.iter().any(|rd| rd.meldingstype == Some(meldingstype))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::payload::PayloadStatus;
    use crate::database::signature::Signature;
    use chrono::DateTime;
    use uuid::Uuid;

    const TRACE_ID: Uuid = Uuid::from_u128(1);

    fn record(meldingstype: Meldingstype) -> RecordDerived {
        RecordDerived {
            topic_id: 1,
            record_partition: 0,
            record_offset: 0,
            signature: Signature::Missing,
            timestamp_delta_ms: None,
            trace_id: Some(TRACE_ID),
            payload_status: PayloadStatus::Ok,
            meldingstype: Some(meldingstype),
            periode_id: Some(Uuid::from_u128(7)),
            arbeidssoeker_id: None,
            derived_at: DateTime::from_timestamp_millis(10_000).unwrap(),
        }
    }

    fn eval(typer: &[Meldingstype]) -> Funn {
        evaluate_ingen_periode_uten_startet(typer.iter().copied().map(record).collect())
    }

    #[test]
    fn tom_trace_gir_ingen_trace_id() {
        assert_eq!(
            evaluate_ingen_periode_uten_startet(vec![]),
            Funn::IngenTraceId
        );
    }

    #[test]
    fn periode_startet_uten_trace_id_gir_ingen_trace_id() {
        let mut r = record(Meldingstype::PeriodeStartet);
        r.trace_id = None;
        assert_eq!(
            evaluate_ingen_periode_uten_startet(vec![r]),
            Funn::IngenTraceId
        );
    }

    #[test]
    fn periode_startet_etter_hendelse_startet_gir_alt_ok() {
        assert_eq!(
            eval(&[Meldingstype::HendelseStartet, Meldingstype::PeriodeStartet]),
            alt_ok(TRACE_ID)
        );
    }

    #[test]
    fn periode_startet_uten_hendelse_startet_gir_feil() {
        assert_eq!(eval(&[Meldingstype::PeriodeStartet]), feil(TRACE_ID));
    }

    #[test]
    fn hendelse_startet_lest_sist_gir_alt_ok() {
        assert_eq!(
            eval(&[Meldingstype::PeriodeStartet, Meldingstype::HendelseStartet]),
            alt_ok(TRACE_ID)
        );
    }

    #[test]
    fn hendelse_startet_uten_periode_startet_er_ikke_aktuell() {
        assert_eq!(eval(&[Meldingstype::HendelseStartet]), Funn::IkkeAktuell);
        assert_eq!(
            eval(&[Meldingstype::Opplysninger, Meldingstype::HendelseStartet]),
            Funn::IkkeAktuell
        );
    }

    #[test]
    fn andre_meldingstyper_er_ikke_aktuelle() {
        for t in [
            Meldingstype::PeriodeAvsluttet,
            Meldingstype::HendelseAvsluttet,
            Meldingstype::Opplysninger,
        ] {
            assert_eq!(
                eval(&[
                    Meldingstype::HendelseStartet,
                    Meldingstype::PeriodeStartet,
                    t
                ]),
                Funn::IkkeAktuell,
                "{t:?}"
            );
        }
    }
}
