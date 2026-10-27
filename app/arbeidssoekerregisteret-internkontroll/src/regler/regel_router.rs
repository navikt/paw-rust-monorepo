use sqlx::PgConnection;

use crate::regler::{
    funn::Funn, hele_trace_er_signert::hele_trace_er_signert,
    ingen_periode_uten_startet::ingen_periode_uten_startet, regel_katalog::RegelId,
};

pub async fn exec(
    conn: &mut PgConnection,
    regel_id: RegelId,
    xid: i64,
) -> Result<Funn, sqlx::Error> {
    match regel_id {
        RegelId::HeleTraceSignert => hele_trace_er_signert(conn, xid).await,
        RegelId::IngenPeriodeUtenStartet => ingen_periode_uten_startet(conn, xid).await,
    }
}
