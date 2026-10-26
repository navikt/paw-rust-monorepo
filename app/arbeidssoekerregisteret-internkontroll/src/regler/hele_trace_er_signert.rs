use sqlx::PgConnection;

use crate::{
    database::{
        record_derived::{RecordDerived, get_record_derived_trace_up_to_xid},
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
    let data = get_record_derived_trace_up_to_xid(conn, xid).await?;
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
