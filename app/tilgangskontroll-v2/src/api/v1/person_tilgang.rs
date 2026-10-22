use crate::model::dto::request::NavAnsattPersonTilgangRequest;
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
use types::identitetsnummer::Identitetsnummer;
use types::nav_ident::NavIdent;

pub(crate) const API_PERSON_TILGANG_PATH: &str = "/api/v1/tilgang/person";

pub(crate) fn routes(
    auth_state: Arc<AuthState>,
    poao_tilgang_client: Arc<PoaoTilgangClient>,
) -> Router {
    Router::new()
        .route(API_PERSON_TILGANG_PATH, post(handle_post))
        .route_layer(oauth2_middleware(auth_state.clone()))
        .route_layer(http_metrics_layer())
        .route_layer(otel_middleware())
        .with_state(RouterState::new(poao_tilgang_client.clone()))
}

#[tracing::instrument(skip_all)]
async fn handle_post(
    State(state): State<RouterState>,
    Json(payload): Json<NavAnsattPersonTilgangRequest>,
) -> Result<Json<TilgangResponse>, ProblemDetails> {
    payload.validate(API_PERSON_TILGANG_PATH)?;
    let tilgang = payload.tilgang;
    let nav_ident = NavIdent::new(payload.nav_ident);
    match Identitetsnummer::new(payload.identitetsnummer) {
        Some(identitetsnummer) => {
            let har_tilgang = state
                .tilgangskontroll
                .nav_ansatt_tilgang_til_person(tilgang, nav_ident, identitetsnummer)
                .await?;
            Ok(Json(TilgangResponse::new(har_tilgang)))
        }
        None => Err(ProblemDetails::validation_error(
            API_PERSON_TILGANG_PATH,
            "Felt 'identitetsnummer' kunne ikke tolkes",
        )),
    }
}
