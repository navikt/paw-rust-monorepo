use crate::api::query_request::{
    QUERY_REQUEST_BODY_LIMIT, SharedQueryRequest, query_request_layer,
};
use crate::logic::query;
use crate::logic::security::policy::KartleggingPolicy;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response::ArbeidsledighetResponse;
use crate::model::state::RouterState;
use axum::extract::State;
use axum::http::Uri;
use axum::routing::post;
use axum::{Json, Router};
use paw_error_handling::problem_details::ProblemDetails;
use paw_oauth2_resource_server::middleware::oauth2_server_layer;
use paw_oauth2_resource_server::state::AuthState;
use paw_observability::http_metrics::http_metrics_layer;
use paw_observability::http_tracing::otel_trace_layer;
use paw_tilgangskontroll::middleware::tilgangskontroll_layer;
use sqlx::PgPool;
use std::sync::Arc;

pub(crate) const API_ARBEIDSLEDIGHET_PATH: &str = "/api/v1/arbeidsledighet";

pub(crate) fn routes(
    pg_pool: PgPool,
    auth_state: Arc<AuthState>,
    policy: Arc<KartleggingPolicy>,
) -> Router {
    Router::new()
        .route(API_ARBEIDSLEDIGHET_PATH, post(finn_arbeidsledighet))
        .route_layer(tilgangskontroll_layer(policy))
        .route_layer(query_request_layer(QUERY_REQUEST_BODY_LIMIT))
        .route_layer(oauth2_server_layer(auth_state.clone()))
        .route_layer(http_metrics_layer())
        .route_layer(otel_trace_layer())
        .with_state(RouterState::new(pg_pool.clone()))
}

#[tracing::instrument(skip_all)]
async fn finn_arbeidsledighet(
    State(state): State<RouterState>,
    uri: Uri,
    SharedQueryRequest(query_request): SharedQueryRequest,
) -> Result<Json<ArbeidsledighetResponse>, ProblemDetails> {
    let path = uri.path().to_owned();

    let mut tx = state.pg_pool.begin().await.map_err(|e| {
        tracing::error!("Kunne ikke starte transaksjon: {}", e);
        ProblemDetails::database_error(&path, "Transaksjon feilet")
    })?;

    let response = match query_request.as_ref() {
        QueryRequest::Identitetsnummer(query) => {
            query.validate(&path)?;
            query::arbeidsledighet_query::finn_for_identitetsnummer_query_request(&mut tx, query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(&path, "Spørring feilet")
                })?
        }
        QueryRequest::TilknyttetKontor(query) => {
            query.validate(&path)?;
            query::arbeidsledighet_query::finn_for_kontortilknytning_query_request(&mut tx, query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(&path, "Spørring feilet")
                })?
        }
    };

    tx.commit().await.map_err(|e| {
        tracing::error!("Kunne ikke commite transaksjon: {}", e);
        ProblemDetails::database_error(&path, "Transaksjon feilet")
    })?;

    tracing::Span::current().record("arbeidssoekere_count", response.arbeidssoekere.len());
    Ok(Json(response))
}
