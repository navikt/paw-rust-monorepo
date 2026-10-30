use sqlx::PgConnection;

use crate::{
    database::record_derived::RecordDerived,
    database::xid8::Xid8,
    regler::{
        funn::Funn, hele_trace_er_signert::hele_trace_er_signert,
        ingen_periode_uten_startet::ingen_periode_uten_startet,
        melding_har_trace_id::melding_har_trace_id, regel_katalog::RegelId,
    },
};

/// Runs one rule for the record written by `xid`.
///
/// Contract for a rule: it derives its own key from `record` (usually with
/// `Nokkel::fra_record`) and returns `IkkeAktuell` when it cannot. It may only
/// read records with xid <= `xid`; `hent_grunnlag` guarantees that.
/// The key it returns must be on the level declared in its `Regel`.
pub async fn exec(
    conn: &mut PgConnection,
    regel_id: RegelId,
    xid: Xid8,
    record: &RecordDerived,
) -> Result<Funn, sqlx::Error> {
    match regel_id {
        RegelId::HeleTraceSignert => hele_trace_er_signert(conn, xid, record).await,
        RegelId::IngenPeriodeUtenStartet => ingen_periode_uten_startet(conn, xid, record).await,
        RegelId::MeldingHarTraceId => Ok(melding_har_trace_id(xid, record)),
    }
}
