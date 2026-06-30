pub(crate) mod docs;
pub(crate) mod kartlegging;
pub(crate) mod oversikt;

use crate::api::docs::api_docs;
use crate::api::kartlegging::finn_kartlegging;
use crate::api::oversikt::finn_oversikt;
use crate::model::state::RouterState;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use axum_health::paw_tracing::add_otel_trace_layer;
use health_and_monitoring::simple_app_state::AppState;
use paw_texas_resource_server::middleware::texas_middleware;
use paw_texas_resource_server::state::AuthState;
use sqlx::PgPool;
use std::sync::Arc;

pub fn build_router(
    app_state: Arc<AppState>,
    //pg_pool: PgPool,
    auth_state: Arc<AuthState>,
) -> Router {
    let health_routes = axum_health::routes(app_state);
    let docs_routes = Router::new().route("/api/docs", get(api_docs));

    /*
    let oversikt_routes = add_otel_trace_layer(
        Router::new()
            .route("/api/v1/oversikt", post(finn_oversikt))
            .route_layer(middleware::from_fn_with_state(
                auth_state.clone(),
                texas_middleware,
            )),
    )
    .with_state(RouterState::new(pg_pool.clone()));
    let kartlegging_routes = add_otel_trace_layer(
        Router::new()
            .route("/api/v1/kartlegging", post(finn_kartlegging))
            .route_layer(middleware::from_fn_with_state(
                auth_state.clone(),
                texas_middleware,
            )),
    )
    .with_state(RouterState::new(pg_pool.clone()));
     */
    health_routes
        .merge(docs_routes)
        //.merge(oversikt_routes)
        //.merge(kartlegging_routes)
}
