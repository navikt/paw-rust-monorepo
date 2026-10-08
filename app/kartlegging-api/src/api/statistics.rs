use crate::logic::query::statistics_query;
use crate::model::dto::request::StatisticsQueryRequest;
use crate::model::dto::response::StatisticsResponse;
use crate::model::state::RouterState;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use paw_error_handling::problem_details::ProblemDetails;
use paw_observability::http_tracing::otel_middleware;
use sqlx::PgPool;

pub const API_STATISTICS_PATH: &str = "/api/v1/statistics";

pub(crate) fn routes(pg_pool: PgPool) -> Router {
    Router::new()
        .route(API_STATISTICS_PATH, get(get_statistics))
        .route(API_STATISTICS_PATH, post(post_statistics))
        .route_layer(otel_middleware())
        .with_state(RouterState::new(pg_pool.clone()))
}

#[tracing::instrument(skip_all)]
async fn get_statistics(
    State(state): State<RouterState>,
) -> Result<Json<StatisticsResponse>, ProblemDetails> {
    let response = fetch_statistics(state.pg_pool, None).await?;
    Ok(Json(response))
}

#[tracing::instrument(skip_all)]
async fn post_statistics(
    State(state): State<RouterState>,
    request: String,
) -> Result<Json<StatisticsResponse>, ProblemDetails> {
    let query_request: StatisticsQueryRequest = serde_json::from_str(&request).map_err(|e| {
        tracing::error!("Feil ved deserialisering av request body: {}", e);
        ProblemDetails::validation_error(API_STATISTICS_PATH, "Ugyldig request body")
    })?;
    let response = fetch_statistics(state.pg_pool, Some(query_request.kontor_id)).await?;
    Ok(Json(response))
}

async fn fetch_statistics(
    pg_pool: PgPool,
    optional_kontor_id: Option<String>,
) -> Result<StatisticsResponse, ProblemDetails> {
    let mut tx = pg_pool.begin().await.map_err(|e| {
        tracing::error!("Kunne ikke starte transaksjon: {}", e);
        ProblemDetails::database_error(API_STATISTICS_PATH, "Transaksjon feilet")
    })?;

    let response = statistics_query::finn(&mut tx, optional_kontor_id)
        .await
        .map_err(|e| {
            tracing::error!("Feil ved spørring: {}", e);
            ProblemDetails::database_error(API_STATISTICS_PATH, "Spørring feilet")
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Kunne ikke commite transaksjon: {}", e);
        ProblemDetails::database_error(API_STATISTICS_PATH, "Transaksjon feilet")
    })?;

    Ok(response)
}
