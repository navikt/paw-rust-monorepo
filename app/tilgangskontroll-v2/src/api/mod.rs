pub(crate) mod docs;
mod v1;

use axum::Router;
use paw_oauth2_resource_server::state::AuthState;
use paw_observability::health::simple_app_state::AppState;
use poao_tilgang_client::client::PoaoTilgangClient;
use std::sync::Arc;

pub fn build_router(
    app_state: Arc<AppState>,
    auth_state: Arc<AuthState>,
    poao_tilgang_client: Arc<PoaoTilgangClient>,
) -> Router {
    let health_routes = paw_observability::server::routes(app_state);
    let docs_routes = docs::routes();
    let person_tilgang_v1_routes =
        v1::person_tilgang::routes(auth_state.clone(), poao_tilgang_client.clone());
    let kontor_tilgang_v1_routes =
        v1::enhet_tilgang::routes(auth_state.clone(), poao_tilgang_client.clone());

    health_routes
        .merge(docs_routes)
        .merge(person_tilgang_v1_routes)
        .merge(kontor_tilgang_v1_routes)
}
