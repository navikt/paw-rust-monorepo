mod app_logic;
mod http_apis;

use crate::http_apis::register_http_apis;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use log::info as log_info;
use paw_rust_base::error_handling::{AppError, GenericAppError};
use std::sync::Arc;
use tracing::{info, instrument};

#[tokio::main]
async fn main() -> Result<(), Box<dyn AppError>> {
    setup_nais_otel()?;
    info!("Starter test app");
    test_trace();
    match run_app().await {
        Ok(()) => println!("Application exited successfully."),
        Err(e) => eprintln!("Application error: {}", e),
    }
    Ok(())
}

#[instrument]
fn test_trace() {
    info!("Kjører tracing::info fra metode merket med #[instrument]");
    log_info!("Kjører log::info fra metode merket med #[instrument]");
}

async fn run_app() -> Result<(), Box<dyn AppError>> {
    let app_state = Arc::new(AppState::new());
    let app_logic = Arc::new(app_logic::AppLogic::new(Arc::from("Hello")));
    let http_server_task = register_http_apis(app_state.clone(), app_logic.clone());
    app_state.set_has_started(true);
    match http_server_task.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(Box::new(GenericAppError {})),
        Err(_) => Err(Box::new(GenericAppError {})),
    }
}
