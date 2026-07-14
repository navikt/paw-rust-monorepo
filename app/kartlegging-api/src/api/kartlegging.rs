use crate::logic::query;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response::KartleggingResponse;
use crate::model::state::RouterState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use paw_error_handling::problem_details::ProblemDetails;
use paw_oauth2_resource_server::middleware::oauth2_middleware;
use paw_oauth2_resource_server::state::AuthState;
use paw_otel_tracing::otel_middleware::otel_middleware;
use sqlx::PgPool;
use std::sync::Arc;

pub const API_KARTLEGGING_PATH: &str = "/api/v1/kartlegging";

pub(crate) fn routes(pg_pool: PgPool, auth_state: Arc<AuthState>) -> Router {
    Router::new()
        .route(API_KARTLEGGING_PATH, post(finn_kartlegging))
        .route_layer(otel_middleware())
        .route_layer(oauth2_middleware(auth_state.clone()))
        .with_state(RouterState::new(pg_pool.clone()))
}

#[tracing::instrument(skip(state, request), fields(arbeidssoekere_count))]
async fn finn_kartlegging(
    State(state): State<RouterState>,
    request: String,
) -> Result<Json<KartleggingResponse>, ProblemDetails> {
    let query_request: QueryRequest = serde_json::from_str(&request).map_err(|e| {
        tracing::error!("Feil ved deserialisering av request body: {}", e);
        ProblemDetails::validation_error(API_KARTLEGGING_PATH, "Ugyldig request body")
    })?;

    let mut tx = state.pg_pool.begin().await.map_err(|e| {
        tracing::error!("Kunne ikke starte transaksjon: {}", e);
        ProblemDetails::database_error(API_KARTLEGGING_PATH, "Transaksjon feilet")
    })?;

    let response = match query_request {
        QueryRequest::Identitetsnummer(query) => {
            query.validate(API_KARTLEGGING_PATH)?;
            query::arbeidssoeker_query::finn_for_identitetsnummer_query_request(&mut tx, &query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(API_KARTLEGGING_PATH, "Spørring feilet")
                })?
        }
        QueryRequest::TilknyttetKontor(query) => {
            query.validate(API_KARTLEGGING_PATH)?;
            query::arbeidssoeker_query::finn_for_kontortilknytning_query_request(&mut tx, &query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(API_KARTLEGGING_PATH, "Spørring feilet")
                })?
        }
    };

    tx.commit().await.map_err(|e| {
        tracing::error!("Kunne ikke commite transaksjon: {}", e);
        ProblemDetails::database_error(API_KARTLEGGING_PATH, "Transaksjon feilet")
    })?;

    tracing::Span::current().record("arbeidssoekere_count", response.arbeidssoekere.len());
    Ok(Json(response))
}
