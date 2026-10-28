use axum::{
    Extension, Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oauth2::principal::Principal;
use paw_oauth2_resource_server::oauth2::{
    config::AuthConfig, middleware::oauth2_server_layer, state::AuthState,
};
use paw_rust_base::config::read_toml_config;
use sqlx::PgPool;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::error;

use crate::database::visualisering::{
    MAKS_ETTERSLEP, ManglerTraceId, RegelStatus, hent_mangler_trace_id_per_topic, hent_max_xid,
    hent_regel_status,
};
use crate::regler::regel_katalog::RegelId;

const VELKOMMEN_HTML: &str = include_str!("velkommen.html");
const HTMX_JS: &str = include_str!("static/htmx.min.js");

/// Auth for visualiseringen. `None` bare når appen er bygget med featuren
/// `local`: alle andre bygg krever innlogging, også om miljøet oppdages feil.
pub async fn auth_state(
    auth_config_toml: &str,
) -> Result<Option<Arc<AuthState>>, Box<dyn std::error::Error>> {
    if cfg!(feature = "local") {
        tracing::warn!("Bygget med feature local: visualiseringen kjører uten auth");
        return Ok(None);
    }
    let config: AuthConfig = read_toml_config(auth_config_toml)?;
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let state = AuthState::new(config, http_client)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(state))
}

/// Alle ruter ligger under `/stats`, som er stien i ingressen. `/internal`
/// nås derfor ikke utenfra.
pub fn routes(pg_pool: PgPool, auth_state: Option<Arc<AuthState>>) -> Router {
    let mut router = Router::new()
        .route("/stats", get(velkommen))
        .route("/stats/", get(velkommen))
        .route("/stats/regler", get(regler))
        .route("/stats/mangler-trace-id", get(mangler_trace_id))
        .route("/stats/static/htmx.min.js", get(htmx));
    if let Some(auth_state) = auth_state {
        router = router.route_layer(oauth2_server_layer(auth_state));
    }
    sikkerhetslag(router).with_state(pg_pool)
}

/// HX-Redirect ved 401 og sikkerhetsheadere. Ligger utenfor auth-laget.
fn sikkerhetslag<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router
        .layer(middleware::from_fn(hx_redirect_ved_401))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; style-src 'self' 'unsafe-inline'"),
        ))
}

/// Når sesjonen er utløpt, laster HTMX hele siden på nytt så Wonderwall
/// kan logge inn igjen. Fast sti, aldri en verdi fra requesten.
async fn hx_redirect_ved_401(request: Request, next: Next) -> Response {
    let er_htmx = request
        .headers()
        .get("HX-Request")
        .is_some_and(|v| v == "true");
    let mut response = next.run(request).await;
    if er_htmx && response.status() == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert("HX-Redirect", HeaderValue::from_static("/stats/"));
    }
    response
}

async fn velkommen(principal: Option<Extension<Principal>>) -> Html<&'static str> {
    if let Some(Extension(Principal::NavAnsatt(ansatt))) = principal {
        tracing::info!(nav_ident = ansatt.ident.as_ref(), "Visualisering åpnet");
    }
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

/// HTML-fragment med antall records uten trace_id per topic.
async fn mangler_trace_id(State(pg_pool): State<PgPool>) -> Result<Html<String>, StatusCode> {
    let mut conn = pg_pool.acquire().await.map_err(intern_feil)?;
    let rader = hent_mangler_trace_id_per_topic(&mut conn)
        .await
        .map_err(intern_feil)?;
    Ok(Html(mangler_trace_id_tabell(&rader)))
}

/// Topic-navn kommer fra databasen og escapes.
fn mangler_trace_id_tabell(rader: &[ManglerTraceId]) -> String {
    if rader.is_empty() {
        return "<p>Ingen records mangler trace_id.</p>".to_string();
    }
    let mut html =
        String::from("<table><thead><tr><th>Topic</th><th>Antall</th></tr></thead><tbody>");
    for r in rader {
        let _ = write!(
            html,
            "<tr><td>{}</td><td>{}</td></tr>",
            escape_html(&r.topic),
            r.antall
        );
    }
    html.push_str("</tbody></table>");
    html
}

fn escape_html(s: &str) -> String {
    let mut ut = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => ut.push_str("&amp;"),
            '<' => ut.push_str("&lt;"),
            '>' => ut.push_str("&gt;"),
            '"' => ut.push_str("&quot;"),
            '\'' => ut.push_str("&#39;"),
            _ => ut.push(c),
        }
    }
    ut
}

fn intern_feil(e: sqlx::Error) -> StatusCode {
    error!("Henting av data til visualisering feilet: {e}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    fn lazy_pool() -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://x@localhost/x")
            .unwrap()
    }

    #[tokio::test]
    async fn uten_auth_gir_side_med_sikkerhetsheadere() {
        let response = routes(lazy_pool(), None)
            .oneshot(Request::get("/stats/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let h = response.headers();
        assert_eq!(h[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert!(h.contains_key(header::CONTENT_SECURITY_POLICY));
    }

    #[tokio::test]
    async fn siden_svarer_under_stats_med_og_uten_skraastrek() {
        let app = routes(lazy_pool(), None);
        for sti in ["/stats", "/stats/", "/stats/static/htmx.min.js"] {
            let response = app
                .clone()
                .oneshot(Request::get(sti).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{sti}");
        }
    }

    #[test]
    fn mangler_trace_id_tabell_tom_og_escapet() {
        assert_eq!(
            mangler_trace_id_tabell(&[]),
            "<p>Ingen records mangler trace_id.</p>"
        );
        let html = mangler_trace_id_tabell(&[ManglerTraceId {
            topic: "<script>&'\"".to_string(),
            antall: 3,
        }]);
        assert!(html.contains("<td>&lt;script&gt;&amp;&#39;&quot;</td><td>3</td>"));
        assert!(!html.contains("<script>"));
    }

    #[tokio::test]
    async fn hx_redirect_bare_for_htmx_ved_401() {
        let app = sikkerhetslag(
            Router::new().route("/regler", get(|| async { StatusCode::UNAUTHORIZED })),
        );
        let kall = |htmx: bool| {
            let mut req = Request::get("/regler");
            if htmx {
                req = req.header("HX-Request", "true");
            }
            app.clone().oneshot(req.body(Body::empty()).unwrap())
        };
        let med = kall(true).await.unwrap();
        assert_eq!(med.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(med.headers()["HX-Redirect"], "/stats/");
        assert!(
            !kall(false)
                .await
                .unwrap()
                .headers()
                .contains_key("HX-Redirect")
        );
    }
}
