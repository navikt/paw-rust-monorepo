use anyhow::Result;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use oversikt_api::routes::build_router;
use oversikt_api::server::{async_task_handler, shutdown_signal_task, web_server_task};
use paw_rust_base::panic_logger::register_panic_logger;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    register_panic_logger();
    setup_nais_otel()?;

    let app_state = Arc::new(simple_app_state::AppState::new());

    let router = build_router(app_state.clone());
    let server_task = web_server_task(router).await;
    let signal_task = shutdown_signal_task();

    app_state.set_has_started(true);

    tokio::select! {
        result = server_task => async_task_handler("Webserver", result),
        signal = signal_task => {
            tracing::info!("Mottok shutdown-signal: {}", signal?);
            Ok(())
        },
    }?;

    Ok(())
}
