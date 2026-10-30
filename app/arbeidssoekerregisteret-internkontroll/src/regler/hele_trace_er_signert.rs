use sqlx::PgConnection;

use crate::{
    database::xid8::Xid8,
    database::{
        record_derived::{RecordDerived, hent_grunnlag},
        signature::Signature,
    },
    regler::{
        alvorlighet::{Alvorlighet, Utfall},
        funn::Funn,
        nokkel::{Nivaa, Nokkel},
        regel_katalog::RegelId,
        regler_task::Regel,
    },
};

const REGEL_VERSJON: i32 = 1;
const NIVAA: Nivaa = Nivaa::Trace;
const IKKE_GYLDIG_SIGNATUR: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Kritisk,
};

pub fn regel_hele_trace_er_signert() -> Regel {
    Regel::new(RegelId::HeleTraceSignert, REGEL_VERSJON, NIVAA)
}

/// Records without trace_id are not relevant: they belong to no trace.
pub async fn hele_trace_er_signert(
    conn: &mut PgConnection,
    xid: Xid8,
    record: &RecordDerived,
) -> Result<Funn, sqlx::Error> {
    let Some(nokkel) = Nokkel::fra_record(xid, record, NIVAA) else {
        return Ok(Funn::IkkeAktuell);
    };
    let data = hent_grunnlag(conn, &nokkel, xid).await?;
    Ok(evaluate_hele_trace_er_signert(nokkel, &data))
}

pub fn evaluate_hele_trace_er_signert(nokkel: Nokkel, data: &[RecordDerived]) -> Funn {
    let problemer = data.iter().any(|rd| {
        !matches!(
            rd.signature,
            Signature::Valid { .. } | Signature::UnsignedBeforeRequired
        )
    });
    if problemer {
        Funn::Feil(nokkel, IKKE_GYLDIG_SIGNATUR)
    } else {
        Funn::AltOk(nokkel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::payload::PayloadStatus;
    use crate::database::signature::SignatureStatus;
    use crate::database::signature::tests::alle_signaturer;
    use chrono::DateTime;
    use uuid::Uuid;

    const TRACE_ID: Uuid = Uuid::from_u128(1);

    fn record(signature: Signature) -> RecordDerived {
        RecordDerived {
            topic_id: 1,
            record_partition: 0,
            record_offset: 0,
            signature,
            timestamp_delta_ms: None,
            trace_id: Some(TRACE_ID),
            payload_status: PayloadStatus::Ok,
            meldingstype: None,
            periode_id: None,
            arbeidssoeker_id: None,
            derived_at: DateTime::from_timestamp_millis(10_000).unwrap(),
        }
    }

    fn gyldig() -> Signature {
        Signature::Valid {
            signing_key_id: "key-a".to_string(),
        }
    }

    fn er_godkjent(signature: &Signature) -> bool {
        [
            SignatureStatus::Valid,
            SignatureStatus::UnsignedBeforeRequired,
        ]
        .contains(&signature.status())
    }

    const NOKKEL: Nokkel = Nokkel::Trace(TRACE_ID);

    fn alt_ok_funn() -> Funn {
        Funn::AltOk(NOKKEL)
    }

    fn feil_funn() -> Funn {
        Funn::Feil(NOKKEL, IKKE_GYLDIG_SIGNATUR)
    }

    fn eval(data: Vec<RecordDerived>) -> Funn {
        evaluate_hele_trace_er_signert(NOKKEL, &data)
    }

    #[test]
    fn record_uten_trace_id_gir_ingen_nokkel() {
        let mut uten_trace = record(gyldig());
        uten_trace.trace_id = None;
        assert_eq!(Nokkel::fra_record(Xid8::new(1), &uten_trace, NIVAA), None);
    }

    #[test]
    fn gyldige_og_usignerte_foer_krav_gir_alt_ok() {
        let funn = eval(vec![
            record(gyldig()),
            record(Signature::UnsignedBeforeRequired),
            record(gyldig()),
        ]);
        assert_eq!(funn, alt_ok_funn());
    }

    #[test]
    fn hver_signaturstatus_alene_gir_forventet_funn() {
        for signature in alle_signaturer() {
            let forventet = if er_godkjent(&signature) {
                alt_ok_funn()
            } else {
                feil_funn()
            };
            assert_eq!(
                eval(vec![record(signature.clone())]),
                forventet,
                "{signature:?}"
            );
        }
    }

    #[test]
    fn en_ugyldig_signatur_hvor_som_helst_i_trace_gir_feil() {
        for posisjon in 0..3 {
            let mut trace = vec![record(gyldig()), record(gyldig()), record(gyldig())];
            trace[posisjon] = record(Signature::Missing);
            assert_eq!(
                eval(trace),
                feil_funn(),
                "ugyldig signatur på posisjon {posisjon}"
            );
        }
    }
}
