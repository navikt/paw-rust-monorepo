pub(crate) mod docs;
pub(crate) mod kartlegging;
pub(crate) mod statistics;

use axum::Router;
use health_and_monitoring::simple_app_state::AppState;
use paw_oauth2_resource_server::state::AuthState;
use sqlx::PgPool;
use std::sync::Arc;

pub fn build_router(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    auth_state: Arc<AuthState>,
) -> Router {
    let health_routes = axum_health::routes(app_state);
    let docs_routes = docs::routes();
    let kartlegging_routes = kartlegging::routes(pg_pool.clone(), auth_state);
    let statistics_routes = statistics::routes(pg_pool.clone());

    health_routes
        .merge(docs_routes)
        .merge(kartlegging_routes)
        .merge(statistics_routes)
}
