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

use crate::database::signature::SignatureStatus;
use crate::database::visualisering::{
    ANTALL_SIGNATURFEIL, MAKS_ETTERSLEP, ManglerTraceId, RegelStatus, Signaturfeil,
    hent_mangler_trace_id_per_topic, hent_max_xid, hent_regel_status, hent_siste_signaturfeil,
};
use crate::regler::regel_katalog::RegelId;

const VELKOMMEN_HTML: &str = include_str!("velkommen.html");
const FANE_OVERSIKT_HTML: &str = include_str!("fane_oversikt.html");
const FANE_SIGNATURFEIL_HTML: &str = include_str!("fane_signaturfeil.html");
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
        .route("/stats/signaturfeil", get(signaturfeil))
        .route(
            "/stats/fane/oversikt",
            get(|| async { Html(FANE_OVERSIKT_HTML) }),
        )
        .route(
            "/stats/fane/signaturfeil",
            get(|| async { Html(FANE_SIGNATURFEIL_HTML) }),
        )
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
    let mut html = String::from(
        "<div class=\"tabell\"><table class=\"mangler-trace-id\"><thead><tr><th>Topic</th><th>Antall</th>\
         <th>Siste tidspunkt (Kafka)</th><th>Siste tidsdelta (ms)</th></tr></thead><tbody>",
    );
    for r in rader {
        let _ = write!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&r.topic),
            r.antall,
            tidspunkt_tekst(r.siste_timestamp_ms),
            r.siste_timestamp_delta_ms
                .map_or("–".to_string(), |d| d.to_string()),
        );
    }
    html.push_str("</tbody></table></div>");
    html
}

/// HTML-fragment med de siste records som feilet signaturvalidering.
async fn signaturfeil(State(pg_pool): State<PgPool>) -> Result<Html<String>, StatusCode> {
    let mut conn = pg_pool.acquire().await.map_err(intern_feil)?;
    let feil = hent_siste_signaturfeil(&mut conn)
        .await
        .map_err(intern_feil)?;
    Ok(Html(signaturfeil_tabell(&feil)))
}

fn signatur_status_tekst(status: SignatureStatus) -> &'static str {
    match status {
        SignatureStatus::Valid => "Gyldig",
        SignatureStatus::SignatureMismatch => "Feil signatur",
        SignatureStatus::MalformedSignature => "Ugyldig signaturformat",
        SignatureStatus::KeyRevoked => "Nøkkel trukket tilbake",
        SignatureStatus::PayloadTooLarge => "For stor payload",
        SignatureStatus::MissingSignature => "Mangler signatur",
        SignatureStatus::UnsignedBeforeRequired => "Usignert før påkrevd",
        SignatureStatus::UnknownKey => "Ukjent nøkkel",
        SignatureStatus::NoRecordTimestamp => "Mangler tidsstempel",
        SignatureStatus::TechnicalError => "Teknisk feil",
    }
}

/// Hva som gikk galt, utledet fra statusen. Detaljer utover dette lagres ikke.
fn signatur_problem(status: SignatureStatus) -> &'static str {
    match status {
        SignatureStatus::Valid => "",
        SignatureStatus::SignatureMismatch => "Signaturen stemmer ikke med innholdet og nøkkelen",
        SignatureStatus::MalformedSignature => "Signaturen kunne ikke leses",
        SignatureStatus::KeyRevoked => "Nøkkelen var trukket tilbake da meldingen ble sendt",
        SignatureStatus::PayloadTooLarge => "Meldingen er for stor til å valideres",
        SignatureStatus::MissingSignature => {
            "Signatur eller nøkkel-ID mangler etter at signatur ble påkrevd"
        }
        SignatureStatus::UnsignedBeforeRequired => "Usignert før signatur ble påkrevd",
        SignatureStatus::UnknownKey => "Nøkkel-ID er ukjent eller ugyldig",
        SignatureStatus::NoRecordTimestamp => {
            "Mangler tidsstempel, så vi kan ikke avgjøre om signatur var påkrevd \
             eller om nøkkelen var gyldig"
        }
        SignatureStatus::TechnicalError => {
            "Feil i nøkkeloppsettet (ugyldig nøkkel, ugyldig tilbaketrekkingsdato, \
             duplikat nøkkel-ID eller lesefeil). Signaturen ble ikke sjekket"
        }
    }
}

fn tidspunkt_tekst(ms: Option<i64>) -> String {
    ms.and_then(chrono::DateTime::from_timestamp_millis)
        .map_or("–".to_string(), |t| {
            t.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string()
        })
}

/// Topic-navn og nøkkel-ID kommer fra databasen og escapes.
fn signaturfeil_tabell(feil: &[Signaturfeil]) -> String {
    if feil.is_empty() {
        return "<p>Ingen records har feilet signaturvalidering.</p>".to_string();
    }
    let mut html = String::new();
    let _ = write!(
        html,
        "<p>De {ANTALL_SIGNATURFEIL} siste, nyeste først.</p>\
         <div class=\"tabell\"><table class=\"signaturfeil\"><thead><tr><th>Topic</th><th>Partisjon</th><th>Offset</th>\
         <th>Tidspunkt (Kafka)</th><th>Status</th><th>Problem</th><th>Nøkkel-ID</th>\
         <th>Trace-ID</th>\
         </tr></thead><tbody>"
    );
    for f in feil {
        let _ = write!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&f.topic),
            f.record_partition,
            f.record_offset,
            tidspunkt_tekst(f.record_timestamp_ms),
            signatur_status_tekst(f.signature_status),
            signatur_problem(f.signature_status),
            f.signing_key_id
                .as_deref()
                .map_or("–".to_string(), escape_html),
            f.trace_id.map_or("–".to_string(), |t| t.to_string()),
        );
    }
    html.push_str("</tbody></table></div>");
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
        "<div class=\"tabell\"><table class=\"regler\"><thead><tr><th>Regel</th><th>Versjon</th><th>Siste xid</th>\
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
    html.push_str("</tbody></table></div>");
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
        for sti in [
            "/stats",
            "/stats/",
            "/stats/static/htmx.min.js",
            "/stats/fane/oversikt",
            "/stats/fane/signaturfeil",
        ] {
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
            siste_timestamp_ms: Some(0),
            siste_timestamp_delta_ms: Some(-42),
        }]);
        assert!(html.contains(
            "<td>&lt;script&gt;&amp;&#39;&quot;</td><td>3</td>\
             <td>1970-01-01 00:00:00.000 UTC</td><td>-42</td>"
        ));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn signaturfeil_tabell_tom_og_escapet() {
        assert_eq!(
            signaturfeil_tabell(&[]),
            "<p>Ingen records har feilet signaturvalidering.</p>"
        );
        let html = signaturfeil_tabell(&[Signaturfeil {
            topic: "<b>".to_string(),
            record_partition: 1,
            record_offset: 2,
            record_timestamp_ms: Some(0),
            signature_status: SignatureStatus::SignatureMismatch,
            signing_key_id: Some("<i>".to_string()),
            trace_id: None,
        }]);
        assert!(html.contains("<td>&lt;b&gt;</td>"));
        assert!(html.contains("<td>&lt;i&gt;</td>"));
        assert!(html.contains("1970-01-01 00:00:00.000 UTC"));
        assert!(html.contains("Feil signatur"));
        assert!(html.contains(signatur_problem(SignatureStatus::SignatureMismatch)));
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
