//! Enkel visualisering for teamet. Serveres av appen selv under `/stats`,
//! som er stien i ingressen. `/internal` nås derfor ikke utenfra.

pub mod auth;
mod html;
mod sikkerhet;

use std::sync::Arc;

use axum::{
    Extension, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    middleware,
    response::{Html, IntoResponse},
    routing::get,
};
use oauth2::principal::Principal;
use paw_oauth2_resource_server::oauth2::{middleware::oauth2_server_layer, state::AuthState};
use sqlx::PgPool;

use crate::features::regler::regel_katalog::RegelId;
use html::{Fane, Valg};

const HTMX_JS: &str = include_str!("static/htmx.min.js");
const APP_CSS: &str = include_str!("static/app.css");

#[derive(Clone)]
pub struct VisState {
    pub pg_pool: PgPool,
    pub hwm_version: i16,
}

pub fn routes(state: VisState, auth_state: Option<Arc<AuthState>>) -> Router {
    let mut router = Router::new()
        .route("/stats", get(side))
        .route("/stats/", get(side))
        .route("/stats/toppfelt", get(toppfelt))
        .route("/stats/static/htmx.min.js", get(htmx))
        .route("/stats/static/app.css", get(css));
    if let Some(auth_state) = auth_state {
        // Sist lagt til kjører først: OAuth2 validerer tokenet, deretter
        // sjekkes det at det er en Nav-ansatt.
        router = router
            .route_layer(middleware::from_fn(auth::krev_nav_ansatt))
            .route_layer(oauth2_server_layer(auth_state));
    }
    sikkerhet::sikkerhetslag(router).with_state(state)
}

/// Hele siden ved vanlig navigasjon, bare fanen ved HTMX-kall. Gjenoppretting
/// av historikk får alltid hele siden.
async fn side(
    State(state): State<VisState>,
    headers: HeaderMap,
    principal: Option<Extension<Principal>>,
    Query(valg): Query<Valg>,
) -> Result<impl IntoResponse, StatusCode> {
    if valg.regel.is_some_and(|r| RegelId::fra_i16(r).is_none()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let er_htmx = headers.get("HX-Request").is_some_and(|v| v == "true");
    let er_gjenoppretting = headers.contains_key("HX-History-Restore-Request");
    let innhold = fane_innhold(&state, &valg).await?;
    let seksjon = html::fane_seksjon(&valg, &innhold);
    let body = if er_htmx && !er_gjenoppretting {
        seksjon
    } else {
        if let Some(Extension(Principal::NavAnsatt(ansatt))) = principal {
            tracing::info!(nav_ident = ansatt.ident.as_ref(), "Visualisering åpnet");
        }
        html::side(&seksjon)
    };
    Ok((
        [(header::VARY, "HX-Request, HX-History-Restore-Request")],
        Html(body),
    ))
}

async fn fane_innhold(_state: &VisState, valg: &Valg) -> Result<String, StatusCode> {
    Ok(match valg.fane {
        Fane::Oversikt | Fane::Funn | Fane::Signatur | Fane::Trace => "<p>Kommer.</p>".to_string(),
    })
}

async fn toppfelt(State(_state): State<VisState>) -> Result<Html<String>, StatusCode> {
    Ok(Html(String::new()))
}

async fn htmx() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], HTMX_JS)
}

async fn css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], APP_CSS)
}

#[cfg(test)]
mod tests;
