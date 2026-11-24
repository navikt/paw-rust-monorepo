//! Enkel visualisering for teamet. Serveres av appen selv under `/stats`,
//! som er stien i ingressen. `/internal` nås derfor ikke utenfra.

pub mod auth;
mod faner;
mod html;
mod sikkerhet;
mod sporringer;

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

async fn fane_innhold(state: &VisState, valg: &Valg) -> Result<String, StatusCode> {
    let mut tx = sporringer::lesetransaksjon(&state.pg_pool)
        .await
        .map_err(intern_feil)?;
    let html = match valg.fane {
        Fane::Oversikt => {
            let regler = sporringer::hent_regel_status(&mut tx, state.hwm_version)
                .await
                .map_err(intern_feil)?;
            let topics = sporringer::hent_topic_status(&mut tx)
                .await
                .map_err(intern_feil)?;
            let stream_ms = topics.iter().filter_map(|t| t.nyeste_stream_ms).max();
            faner::oversikt(&regler, &topics, stream_ms)
        }
        Fane::Funn => {
            let antall = sporringer::hent_funn_antall(&mut tx)
                .await
                .map_err(intern_feil)?;
            let regel = valg.regel.and_then(RegelId::fra_i16);
            let rader = sporringer::hent_funn(&mut tx, regel, valg.alvorlighet)
                .await
                .map_err(intern_feil)?;
            faner::funn(valg, &antall, &rader)
        }
        Fane::Signatur => {
            let antall = sporringer::hent_signatur_antall(&mut tx)
                .await
                .map_err(intern_feil)?;
            let feil = sporringer::hent_siste_signaturfeil(&mut tx)
                .await
                .map_err(intern_feil)?;
            faner::signatur(&antall, &feil)
        }
        Fane::Trace => {
            let rader = sporringer::hent_mangler_trace_id(&mut tx)
                .await
                .map_err(intern_feil)?;
            faner::trace(&rader)
        }
    };
    Ok(html)
}

async fn toppfelt(State(state): State<VisState>) -> Result<Html<String>, StatusCode> {
    let mut tx = sporringer::lesetransaksjon(&state.pg_pool)
        .await
        .map_err(intern_feil)?;
    let toppfelt = sporringer::hent_toppfelt(&mut tx)
        .await
        .map_err(intern_feil)?;
    Ok(Html(faner::toppfelt(&toppfelt)))
}

fn intern_feil(e: sqlx::Error) -> StatusCode {
    tracing::error!("Henting av data til visualisering feilet: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn htmx() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], HTMX_JS)
}

async fn css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], APP_CSS)
}

#[cfg(test)]
mod tests;
