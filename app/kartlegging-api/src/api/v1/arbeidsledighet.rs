use crate::logic::query;
use crate::logic::security::policy::KartleggingPolicy;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response::ArbeidsledighetResponse;
use crate::model::state::RouterState;
use axum::extract::{FromRequest, Request, State};
use axum::routing::post;
use axum::{Json, Router};
use paw_error_handling::problem_details::ProblemDetails;
use paw_oauth2_resource_server::middleware::oauth2_middleware;
use paw_oauth2_resource_server::state::AuthState;
use paw_observability::http_metrics::http_metrics_layer;
use paw_observability::http_tracing::otel_middleware;
use paw_tilgangskontroll::middleware::tilgangskontroll;
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
        .route_layer(tilgangskontroll(policy))
        .route_layer(oauth2_middleware(auth_state.clone()))
        .route_layer(http_metrics_layer())
        .route_layer(otel_middleware())
        .with_state(RouterState::new(pg_pool.clone()))
}

#[tracing::instrument(skip_all)]
async fn finn_arbeidsledighet(
    State(state): State<RouterState>,
    request: Request,
) -> Result<Json<ArbeidsledighetResponse>, ProblemDetails> {
    let path = request.uri().path().to_owned();
    let Json(query_request) = Json::<QueryRequest>::from_request(request, &state)
        .await
        .map_err(|e| {
            tracing::error!("Feil ved deserialisering av request body: {}", e);
            ProblemDetails::validation_error(&path, "Ugyldig request body")
        })?;

    let mut tx = state.pg_pool.begin().await.map_err(|e| {
        tracing::error!("Kunne ikke starte transaksjon: {}", e);
        ProblemDetails::database_error(&path, "Transaksjon feilet")
    })?;

    let response = match query_request {
        QueryRequest::Identitetsnummer(query) => {
            query.validate(&path)?;
            query::arbeidsledighet_query::finn_for_identitetsnummer_query_request(&mut tx, &query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(&path, "Spørring feilet")
                })?
        }
        QueryRequest::TilknyttetKontor(query) => {
            query.validate(&path)?;
            query::arbeidsledighet_query::finn_for_kontortilknytning_query_request(&mut tx, &query)
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
