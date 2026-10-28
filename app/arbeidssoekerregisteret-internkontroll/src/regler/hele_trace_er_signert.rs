use sqlx::PgConnection;

use crate::{
    database::{
        record_derived::{RecordDerived, get_at_xid_including_preceding_trace_records},
        signature::Signature,
    },
    regler::{funn::Funn, regel_katalog::RegelId, regler_task::Regel},
};

const REGEL_VERSJON: i32 = 1;
const IKKE_GYLDIG_SIGNATUR: i16 = 1;

pub fn regel_hele_trace_er_signert() -> Regel {
    Regel::new(RegelId::HeleTraceSignert, REGEL_VERSJON)
}

fn alt_ok(trace_id: uuid::Uuid) -> Funn {
    Funn::AltOk {
        regel_id: RegelId::HeleTraceSignert,
        regel_versjon: REGEL_VERSJON,
        trace_id,
    }
}

fn feil(trace_id: uuid::Uuid) -> Funn {
    Funn::Feil(crate::regler::funn::Detaljer {
        regel_id: RegelId::HeleTraceSignert,
        regel_versjon: REGEL_VERSJON,
        trace_id,
        utfall: IKKE_GYLDIG_SIGNATUR,
    })
}

pub async fn hele_trace_er_signert(conn: &mut PgConnection, xid: i64) -> Result<Funn, sqlx::Error> {
    let data = get_at_xid_including_preceding_trace_records(conn, xid).await?;
    Ok(evaluate_hele_trace_er_signert(data))
}

pub fn evaluate_hele_trace_er_signert(data: Vec<RecordDerived>) -> Funn {
    let Some(trace_id) = data.first().and_then(|r| r.trace_id) else {
        return Funn::IngenTraceId;
    };
    let problemer = data.iter().any(|rd| {
        !matches!(
            rd.signature,
            Signature::Valid { .. } | Signature::UnsignedBeforeRequired
        )
    });
    if problemer {
        feil(trace_id)
    } else {
        alt_ok(trace_id)
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

    fn alt_ok_funn() -> Funn {
        Funn::AltOk {
            regel_id: RegelId::HeleTraceSignert,
            regel_versjon: REGEL_VERSJON,
            trace_id: TRACE_ID,
        }
    }

    fn feil_funn() -> Funn {
        Funn::Feil(crate::regler::funn::Detaljer {
            regel_id: RegelId::HeleTraceSignert,
            regel_versjon: REGEL_VERSJON,
            trace_id: TRACE_ID,
            utfall: IKKE_GYLDIG_SIGNATUR,
        })
    }

    #[test]
    fn tom_trace_gir_ingen_trace_id() {
        assert_eq!(evaluate_hele_trace_er_signert(vec![]), Funn::IngenTraceId);
    }

    #[test]
    fn record_uten_trace_id_gir_ingen_trace_id() {
        let mut uten_trace = record(gyldig());
        uten_trace.trace_id = None;
        assert_eq!(
            evaluate_hele_trace_er_signert(vec![uten_trace]),
            Funn::IngenTraceId
        );
    }

    #[test]
    fn gyldige_og_usignerte_foer_krav_gir_alt_ok() {
        let funn = evaluate_hele_trace_er_signert(vec![
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
                evaluate_hele_trace_er_signert(vec![record(signature.clone())]),
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
                evaluate_hele_trace_er_signert(trace),
                feil_funn(),
                "ugyldig signatur på posisjon {posisjon}"
            );
        }
    }
}
