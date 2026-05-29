use axum::Router;
use health_and_monitoring::simple_app_state::AppState;
use std::sync::Arc;

pub fn build_router(app_state: Arc<AppState>) -> Router {
    let health_routes = axum_health::routes(app_state);
    health_routes
}
