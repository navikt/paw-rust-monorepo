pub(crate) mod docs;
mod v1;

use crate::api::v1::{arbeidsledighet, kartlegging, statistics};
use axum::Router;
use paw_oauth2_resource_server::state::AuthState;
use paw_observability::health::simple_app_state::AppState;
use sqlx::PgPool;
use std::sync::Arc;

pub fn build_router(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    auth_state: Arc<AuthState>,
) -> Router {
    let health_routes = paw_observability::server::routes(app_state);
    let docs_routes = docs::routes();
    let kartlegging_routes = kartlegging::routes(pg_pool.clone(), auth_state.clone());
    let arbeidsledighet_routes = arbeidsledighet::routes(pg_pool.clone(), auth_state.clone());
    let statistics_routes = statistics::routes(pg_pool.clone());

    health_routes
        .merge(docs_routes)
        .merge(kartlegging_routes)
        .merge(arbeidsledighet_routes)
        .merge(statistics_routes)
}
