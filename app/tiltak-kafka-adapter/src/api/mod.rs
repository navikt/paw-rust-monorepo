use axum::Router;
use paw_observability::health::simple_app_state::AppState;
use std::sync::Arc;

pub fn build_router(app_state: Arc<AppState>) -> Router {
    let health_routes = paw_observability::server::routes(app_state);
    health_routes
}
