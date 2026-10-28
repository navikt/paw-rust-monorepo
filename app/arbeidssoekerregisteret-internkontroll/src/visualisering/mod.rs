use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use sqlx::PgPool;
use std::fmt::Write;
use tracing::error;

use crate::database::visualisering::{
    MAKS_ETTERSLEP, RegelStatus, hent_max_xid, hent_regel_status,
};
use crate::regler::regel_katalog::RegelId;

const VELKOMMEN_HTML: &str = include_str!("velkommen.html");
const HTMX_JS: &str = include_str!("static/htmx.min.js");

pub fn routes(pg_pool: PgPool) -> Router {
    Router::new()
        .route("/", get(velkommen))
        .route("/regler", get(regler))
        .route("/static/htmx.min.js", get(htmx))
        .with_state(pg_pool)
}

async fn velkommen() -> Html<&'static str> {
    Html(VELKOMMEN_HTML)
}

async fn htmx() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], HTMX_JS)
}

/// HTML-fragment med regelstatus. Hentes av HTMX ved lasting og hvert 30. sekund.
async fn regler(State(pg_pool): State<PgPool>) -> Result<Html<String>, StatusCode> {
    let mut conn = pg_pool.acquire().await.map_err(intern_feil)?;
    let status = hent_regel_status(&mut conn).await.map_err(intern_feil)?;
    let max_xid = hent_max_xid(&mut conn).await.map_err(intern_feil)?;
    Ok(Html(regel_tabell(&status, max_xid)))
}

fn intern_feil(e: sqlx::Error) -> StatusCode {
    error!("Henting av regelstatus feilet: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn regel_navn(regel: RegelId) -> &'static str {
    match regel {
        RegelId::HeleTraceSignert => "Hele trace er signert",
        RegelId::IngenPeriodeUtenStartet => "Ingen periode uten startet",
    }
}

/// Alle verdier er tall eller faste tekster, så de trenger ikke HTML-escaping.
fn regel_tabell(status: &[RegelStatus], max_xid: Option<i64>) -> String {
    let mut html = String::new();
    let max_xid = max_xid.map_or("–".to_string(), |x| x.to_string());
    let _ = write!(html, "<p>Høyeste xid i record_derived: {max_xid}</p>");
    if status.is_empty() {
        html.push_str("<p>Ingen regler har registrert fremdrift ennå.</p>");
        return html;
    }
    html.push_str(
        "<table><thead><tr><th>Regel</th><th>Versjon</th><th>Siste xid</th>\
         <th>Records bak</th><th>Funn</th></tr></thead><tbody>",
    );
    for s in status {
        let etter = if s.etter >= MAKS_ETTERSLEP {
            format!("≥ {}", MAKS_ETTERSLEP)
        } else {
            s.etter.to_string()
        };
        let _ = write!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            regel_navn(s.regel),
            s.regelversjon,
            s.siste_xid,
            etter,
            s.antall_funn
        );
    }
    html.push_str("</tbody></table>");
    html
}
