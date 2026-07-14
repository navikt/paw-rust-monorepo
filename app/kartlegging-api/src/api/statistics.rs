use crate::logic::query::statistics_query;
use crate::model::dto::response::StatisticsResponse;
use crate::model::state::RouterState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use paw_error_handling::problem_details::ProblemDetails;
use paw_otel_tracing::otel_middleware::otel_middleware;
use sqlx::PgPool;

pub const API_STATISTICS_PATH: &str = "/api/v1/statistics";

pub(crate) fn routes(pg_pool: PgPool) -> Router {
    Router::new()
        .route(API_STATISTICS_PATH, get(finn_statistics))
        .route_layer(otel_middleware())
        .with_state(RouterState::new(pg_pool.clone()))
}

#[tracing::instrument(skip(state))]
async fn finn_statistics(
    State(state): State<RouterState>,
) -> Result<Json<StatisticsResponse>, ProblemDetails> {
    let mut tx = state.pg_pool.begin().await.map_err(|e| {
        tracing::error!("Kunne ikke starte transaksjon: {}", e);
        ProblemDetails::database_error(API_STATISTICS_PATH, "Transaksjon feilet")
    })?;

    let response = statistics_query::finn(&mut tx).await.map_err(|e| {
        tracing::error!("Feil ved spørring: {}", e);
        ProblemDetails::database_error(API_STATISTICS_PATH, "Spørring feilet")
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Kunne ikke commite transaksjon: {}", e);
        ProblemDetails::database_error(API_STATISTICS_PATH, "Transaksjon feilet")
    })?;
    Ok(Json(response))
}
