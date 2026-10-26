use errors::app::AppError;
use paw_oauth2_resource_server::oauth2::state::AuthState;
use paw_observability::health::simple_app_state;
use paw_observability::otel::setup_otel;
use paw_rust_base::panic_logger::register_panic_logger;
use poao_tilgang_client::client::PoaoTilgangClient;
use reqwest::Client;
use std::sync::Arc;
use texas_client::token_client::create_token_client;
use tilgangskontroll_v2::api::build_router;
use tilgangskontroll_v2::config::{
    HTTP_TIMEOUT, read_auth_config, read_otel_tracing_config, read_poao_tilgang_client_config,
    read_token_client_config,
};
use tilgangskontroll_v2::server::{
    async_task_handler, shutdown_handler, shutdown_signal_task, web_server_task,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    register_panic_logger();

    let otel_tracing_config = read_otel_tracing_config()?;
    let auth_config = read_auth_config()?;
    let token_client_config = read_token_client_config()?;
    let poao_tilgang_client_config = read_poao_tilgang_client_config()?;

    setup_otel(otel_tracing_config)?;

    let http_client = Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|_| AppError::AppInitFailed("Kunne ikke opprette HTTP-klient".to_string()))?;

    let app_state = Arc::new(simple_app_state::AppState::new());
    let auth_state = AuthState::new(auth_config, http_client.clone())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let token_client = Arc::new(create_token_client(
        token_client_config,
        http_client.clone(),
    ));
    let poao_tilgang_client = Arc::new(PoaoTilgangClient::from_config(
        poao_tilgang_client_config,
        http_client.clone(),
        token_client.clone(),
    ));

    let router = build_router(app_state.clone(), auth_state, poao_tilgang_client);
    let server_task = web_server_task(router).await;

    let signal_task = shutdown_signal_task();

    app_state.set_has_started(true);

    tokio::select! {
        result = server_task => async_task_handler("Webserver", result),
        signal = signal_task => shutdown_handler(signal),
    }?;

    Ok(())
}
