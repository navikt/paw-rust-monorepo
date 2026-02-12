mod pdl_query;

use std::{error::Error, sync::Arc};

use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use paw_rust_base::{
    error_handling::{AppError, GenericAppError},
    panic_logger::register_panic_logger,
};
use tokio::{
    signal::{unix::SignalKind, unix::signal},
    task::JoinHandle,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn AppError>> {
    register_panic_logger();
    setup_nais_otel()?;
    let reqwest_client = reqwest::Client::new();
    let token_client = texas_client::token_client::create_token_client(reqwest_client)?;
    let app_state = Arc::new(simple_app_state::AppState::new());
    let health_routes = axum_health::routes(app_state);
    let web_server_task: JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
            axum::serve(listener, health_routes).await?;
            Ok(())
        });
    let signal = get_shutdown_signal()
        .await
        .map_err(|err| GenericAppError {
            message: format!("Feil ved lytting etter shutdown signal: {}", err),
        })?;
    tracing::info!("Avslutter etter at {} ble registrert", signal);
    Ok(())
}

async fn get_shutdown_signal() -> Result<String, Box<dyn Error>> {
    let mut term_signal = signal(SignalKind::terminate())?;
    let mut interrupt_signal = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term_signal.recv() => Ok("SIGTERM".to_string()),
        _ = interrupt_signal.recv() => Ok("SIGINT".to_string())
    }
}
