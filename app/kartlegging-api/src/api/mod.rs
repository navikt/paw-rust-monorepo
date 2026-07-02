pub(crate) mod docs;
pub(crate) mod kartlegging;

use crate::api::docs::api_docs_routes;
use crate::api::kartlegging::kartlegging_routes;
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
    let docs_routes = api_docs_routes();
    let kartlegging_routes = kartlegging_routes(pg_pool, auth_state);

    health_routes.merge(docs_routes).merge(kartlegging_routes)
}
