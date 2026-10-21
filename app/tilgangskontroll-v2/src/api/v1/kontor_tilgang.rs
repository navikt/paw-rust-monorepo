use crate::model::dto::request::NavAnsattKontorTilgangRequest;
use crate::model::dto::response::TilgangResponse;
use crate::model::state::RouterState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use paw_error_handling::problem_details::ProblemDetails;
use paw_oauth2_resource_server::middleware::oauth2_middleware;
use paw_oauth2_resource_server::state::AuthState;
use paw_observability::http_metrics::http_metrics_layer;
use paw_observability::http_tracing::otel_middleware;
use poao_tilgang_client::client::PoaoTilgangClient;
use std::sync::Arc;
use types::nav_ident::NavIdent;
use types::nav_kontor_id::NavKontorId;

pub(crate) const API_KONTOR_TILGANG_PATH: &str = "/api/v1/tilgang/kontor";

pub(crate) fn routes(
    auth_state: Arc<AuthState>,
    poao_tilgang_client: Arc<PoaoTilgangClient>,
) -> Router {
    Router::new()
        .route(API_KONTOR_TILGANG_PATH, post(handle_post))
        .route_layer(oauth2_middleware(auth_state.clone()))
        .route_layer(http_metrics_layer())
        .route_layer(otel_middleware())
        .with_state(RouterState::new(poao_tilgang_client.clone()))
}

#[tracing::instrument(skip_all)]
async fn handle_post(
    State(state): State<RouterState>,
    Json(payload): Json<NavAnsattKontorTilgangRequest>,
) -> Result<Json<TilgangResponse>, ProblemDetails> {
    let nav_ident = NavIdent::new(payload.nav_ident);
    let kontor_id = NavKontorId::new(payload.kontor_id);
    let har_tilgang = state
        .tilgangskontroll
        .nav_ansatt_tilgang_til_kontor(nav_ident, kontor_id)
        .await?;
    Ok(Json(TilgangResponse::new(har_tilgang)))
}
