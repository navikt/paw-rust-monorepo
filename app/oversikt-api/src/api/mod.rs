pub(crate) mod docs;
pub(crate) mod oversikt;

use crate::api::docs::api_docs;
use crate::api::oversikt::finn_oversikt;
use crate::model::context::AppContext;
use axum::routing::{get, post};
use axum::Router;
use axum_health::paw_tracing::add_otel_trace_layer;
use health_and_monitoring::simple_app_state::AppState;
use std::sync::Arc;

pub fn build_router(app_state: Arc<AppState>, app_context: AppContext) -> Router {
    let health_routes = axum_health::routes(app_state);
    let oversikt_routes =
        add_otel_trace_layer(Router::new().route("/api/v1/oversikt", post(finn_oversikt)))
            .with_state(app_context);
    let docs_routes = Router::new().route("/api/docs", get(api_docs));
    health_routes.merge(oversikt_routes).merge(docs_routes)
}
