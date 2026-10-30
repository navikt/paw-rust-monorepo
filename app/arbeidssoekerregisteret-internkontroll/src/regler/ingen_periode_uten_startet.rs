use sqlx::PgConnection;

use crate::{
    database::xid8::Xid8,
    database::{
        payload::Meldingstype,
        record_derived::{RecordDerived, hent_grunnlag},
    },
    regler::{
        alvorlighet::{Alvorlighet, Utfall},
        funn::Funn,
        nokkel::{Nivaa, Nokkel},
        regel_katalog::RegelId,
        regelversjon::Regelversjon,
        regler_task::Regel,
    },
};

const REGELVERSJON: Regelversjon = Regelversjon::new(1);
const NIVAA: Nivaa = Nivaa::Trace;
const PERIODE_STARTET_UTEN_HENDELSE_STARTET: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Kritisk,
};

pub fn regel_ingen_periode_uten_startet() -> Regel {
    Regel::new(RegelId::IngenPeriodeUtenStartet, REGELVERSJON, NIVAA)
}

/// Records without trace_id are not relevant: they belong to no trace.
pub async fn ingen_periode_uten_startet(
    conn: &mut PgConnection,
    xid: Xid8,
    record: &RecordDerived,
) -> Result<Funn, sqlx::Error> {
    let Some(nokkel) = Nokkel::fra_record(xid, record, NIVAA) else {
        return Ok(Funn::IkkeAktuell);
    };
    let data = hent_grunnlag(conn, &nokkel, xid).await?;
    Ok(evaluate_ingen_periode_uten_startet(nokkel, &data))
}

/// `data` is the trace up to and including the record being evaluated, in xid
/// order, so the evaluated record is the last one.
///
/// - `PeriodeStartet` is OK only if a `HendelseStartet` came earlier in the trace.
/// - `HendelseStartet` after a `PeriodeStartet` in the trace is OK: this AltOk
///   replaces the Feil written when the `PeriodeStartet` was read first.
///   Without a `PeriodeStartet` (for example a duplicate) it is not relevant.
/// - Every other message type is not relevant.
pub fn evaluate_ingen_periode_uten_startet(nokkel: Nokkel, data: &[RecordDerived]) -> Funn {
    let Some(gjeldende) = data.last() else {
        return Funn::IkkeAktuell;
    };
    match gjeldende.meldingstype {
        Some(Meldingstype::HendelseStartet) => {
            if finnes(data, Meldingstype::PeriodeStartet) {
                Funn::AltOk(nokkel)
            } else {
                Funn::IkkeAktuell
            }
        }
        Some(Meldingstype::PeriodeStartet) => {
            if finnes(data, Meldingstype::HendelseStartet) {
                Funn::AltOk(nokkel)
            } else {
                Funn::Feil(nokkel, PERIODE_STARTET_UTEN_HENDELSE_STARTET)
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

    const NOKKEL: Nokkel = Nokkel::Trace(TRACE_ID);

    fn eval(typer: &[Meldingstype]) -> Funn {
        let data: Vec<_> = typer.iter().copied().map(record).collect();
        evaluate_ingen_periode_uten_startet(NOKKEL, &data)
    }

    fn alt_ok(_: Uuid) -> Funn {
        Funn::AltOk(NOKKEL)
    }

    fn feil(_: Uuid) -> Funn {
        Funn::Feil(NOKKEL, PERIODE_STARTET_UTEN_HENDELSE_STARTET)
    }

    #[test]
    fn tomt_grunnlag_er_ikke_aktuelt() {
        assert_eq!(
            evaluate_ingen_periode_uten_startet(NOKKEL, &[]),
            Funn::IkkeAktuell
        );
    }

    #[test]
    fn periode_startet_uten_trace_id_gir_ingen_nokkel() {
        let mut r = record(Meldingstype::PeriodeStartet);
        r.trace_id = None;
        assert_eq!(Nokkel::fra_record(Xid8::new(1), &r, NIVAA), None);
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
