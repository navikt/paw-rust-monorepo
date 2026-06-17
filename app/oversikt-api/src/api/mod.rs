pub(crate) mod docs;
pub(crate) mod oversikt;

use crate::api::docs::api_docs;
use crate::api::oversikt::finn_oversikt;
use crate::model::context::AppContext;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use axum_health::paw_tracing::add_otel_trace_layer;
use health_and_monitoring::simple_app_state::AppState;
use paw_texas_resource_server::middleware::texas_middleware;
use paw_texas_resource_server::state::AuthState;
use std::sync::Arc;

pub fn build_router(
    app_state: Arc<AppState>,
    app_context: AppContext,
    auth_state: Arc<AuthState>,
) -> Router {
    let health_routes = axum_health::routes(app_state);
    let docs_routes = Router::new().route("/api/docs", get(api_docs));
    let oversikt_routes = add_otel_trace_layer(
        Router::new()
            .route("/api/v1/oversikt", post(finn_oversikt))
            .route_layer(middleware::from_fn_with_state(auth_state, texas_middleware)),
    )
    .with_state(app_context);
    health_routes.merge(docs_routes).merge(oversikt_routes)
}
