pub(crate) mod docs;
mod v1;

use crate::logic::security::policy::KartleggingPolicy;
use axum::Router;
use paw_oauth2_resource_server::state::AuthState;
use paw_observability::health::simple_app_state::AppState;
use paw_tilgangskontroll_client::client::PawTilgangskontrollClient;
use sqlx::PgPool;
use std::sync::Arc;

pub fn build_router(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    auth_state: Arc<AuthState>,
    paw_tilgangskontroll_client: Arc<PawTilgangskontrollClient>,
) -> Router {
    let policy = Arc::new(KartleggingPolicy::new(paw_tilgangskontroll_client));

    let health_routes = paw_observability::server::routes(app_state);
    let docs_routes = docs::routes();
    let kartlegging_v1_routes =
        v1::kartlegging::routes(pg_pool.clone(), auth_state.clone(), policy.clone());
    let arbeidsledighet_v1_routes =
        v1::arbeidsledighet::routes(pg_pool.clone(), auth_state.clone(), policy.clone());
    let statistics_v1_routes = v1::statistics::routes(pg_pool.clone());

    health_routes
        .merge(docs_routes)
        .merge(kartlegging_v1_routes)
        .merge(arbeidsledighet_v1_routes)
        .merge(statistics_v1_routes)
}
