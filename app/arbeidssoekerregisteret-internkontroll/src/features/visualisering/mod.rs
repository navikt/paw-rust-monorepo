//! Enkel visualisering for teamet. Serveres av appen selv under `/stats`,
//! som er stien i ingressen. `/internal` nås derfor ikke utenfra.
//!
//! Rutene her skal bare vise aggregater og topic/partisjon/offset, aldri
//! personopplysninger som `periode_id` eller `arbeidssoeker_id`. Derfor logges
//! bare visning av hele siden, ikke fragmentene. Sensitive data skal ligge bak
//! egne endepunkter med detaljert logging av hvem som så hva.

pub mod auth;
mod faner;
mod html;
mod sikkerhet;
mod sporringer;

use std::sync::Arc;

use axum::{
    Extension, Form, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use maud::Markup;
use oauth2::principal::Principal;
use paw_oauth2_resource_server::oauth2::{middleware::oauth2_server_layer, state::AuthState};
use sqlx::{PgConnection, PgPool};

use crate::features::regler::regel_katalog::RegelId;
use html::{Fane, Tema, TemaSkjema, Valg};

const OPPDATER_JS: &str = include_str!("static/oppdater.js");
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
        .route("/stats/fragment/fane", get(fane_fragment))
        .route("/stats/fragment/toppfelt", get(toppfelt_fragment))
        .route("/stats/tema", post(lagre_tema))
        .route("/stats/static/oppdater.js", get(js))
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

fn valider(valg: &Valg) -> Result<(), StatusCode> {
    match valg.regel {
        Some(r) if RegelId::fra_i16(r).is_none() => Err(StatusCode::BAD_REQUEST),
        _ => Ok(()),
    }
}

/// Hele siden. Hver visning logges med NAV-ident. Automatisk oppdatering går
/// til fragmentrutene og logges ikke.
async fn side(
    State(state): State<VisState>,
    principal: Option<Extension<Principal>>,
    headers: HeaderMap,
    Query(valg): Query<Valg>,
) -> Result<Markup, StatusCode> {
    valider(&valg)?;
    if let Some(Extension(Principal::NavAnsatt(ansatt))) = &principal {
        tracing::info!(
            nav_ident = ansatt.ident.as_ref(),
            fane = valg.fane.as_str(),
            "Visualisering vist"
        );
    }
    let mut tx = sporringer::lesetransaksjon(&state.pg_pool)
        .await
        .map_err(intern_feil)?;
    let toppfelt_data = sporringer::hent_toppfelt(&mut tx)
        .await
        .map_err(intern_feil)?;
    let toppfelt = faner::toppfelt(&toppfelt_data, chrono::Utc::now());
    let innhold = fane_innhold(&mut tx, state.hwm_version, &valg, toppfelt_data.stream_ms).await?;
    let tema = Tema::fra_cookie_header(headers.get(header::COOKIE).and_then(|v| v.to_str().ok()));
    Ok(html::side(&valg, tema, &toppfelt_data, toppfelt, innhold))
}

/// Lagrer temavalget i en cookie og sender tilbake til siden. Målet er fast,
/// så ruten kan ikke brukes til å sende brukeren videre til andre steder.
/// Ugyldig skjema gir 400.
async fn lagre_tema(
    skjema: Result<Form<TemaSkjema>, axum::extract::rejection::FormRejection>,
) -> Result<impl IntoResponse, StatusCode> {
    let Form(TemaSkjema { tema }) = skjema.map_err(|_| StatusCode::BAD_REQUEST)?;
    let cookie = format!(
        "tema={}; Path=/stats; Max-Age=31536000; SameSite=Lax; HttpOnly; Secure",
        tema.as_str()
    );
    Ok((
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, "/stats/".to_string()),
            (header::SET_COOKIE, cookie),
        ],
    ))
}

async fn fane_fragment(
    State(state): State<VisState>,
    Query(valg): Query<Valg>,
) -> Result<Markup, StatusCode> {
    valider(&valg)?;
    let mut tx = sporringer::lesetransaksjon(&state.pg_pool)
        .await
        .map_err(intern_feil)?;
    let stream_ms = if valg.fane == Fane::Oversikt {
        sporringer::hent_toppfelt(&mut tx)
            .await
            .map_err(intern_feil)?
            .stream_ms
    } else {
        None
    };
    fane_innhold(&mut tx, state.hwm_version, &valg, stream_ms).await
}

async fn toppfelt_fragment(State(state): State<VisState>) -> Result<Markup, StatusCode> {
    let mut tx = sporringer::lesetransaksjon(&state.pg_pool)
        .await
        .map_err(intern_feil)?;
    let toppfelt = sporringer::hent_toppfelt(&mut tx)
        .await
        .map_err(intern_feil)?;
    Ok(faner::toppfelt(&toppfelt, chrono::Utc::now()))
}

async fn fane_innhold(
    tx: &mut PgConnection,
    hwm_version: i16,
    valg: &Valg,
    stream_ms: Option<i64>,
) -> Result<Markup, StatusCode> {
    let html = match valg.fane {
        Fane::Oversikt => {
            let regler = sporringer::hent_regel_status(&mut *tx, hwm_version)
                .await
                .map_err(intern_feil)?;
            let topics = sporringer::hent_topic_status(&mut *tx)
                .await
                .map_err(intern_feil)?;
            faner::oversikt(&regler, &topics, stream_ms)
        }
        Fane::Funn => {
            let antall = sporringer::hent_funn_antall(&mut *tx)
                .await
                .map_err(intern_feil)?;
            let regel = valg.regel.and_then(RegelId::fra_i16);
            let rader = sporringer::hent_funn(&mut *tx, regel, valg.alvorlighet)
                .await
                .map_err(intern_feil)?;
            faner::funn(valg, &antall, &rader)
        }
        Fane::Signatur => {
            let antall = sporringer::hent_signatur_antall(&mut *tx)
                .await
                .map_err(intern_feil)?;
            let feil = sporringer::hent_siste_signaturfeil(&mut *tx)
                .await
                .map_err(intern_feil)?;
            faner::signatur(&antall, &feil)
        }
        Fane::Trace => {
            let rader = sporringer::hent_mangler_trace_id(&mut *tx)
                .await
                .map_err(intern_feil)?;
            faner::trace(&rader)
        }
    };
    Ok(html)
}

fn intern_feil(e: sqlx::Error) -> StatusCode {
    tracing::error!("Henting av data til visualisering feilet: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn js() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/javascript")], OPPDATER_JS)
}

async fn css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], APP_CSS)
}

#[cfg(test)]
mod tests;
