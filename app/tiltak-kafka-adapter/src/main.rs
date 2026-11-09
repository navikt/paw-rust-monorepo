use paw_observability::health::simple_app_state;
use paw_observability::otel::setup_otel;
use paw_rust_base::panic_logger::register_panic_logger;
use std::sync::Arc;
use tiltak_kafka_adapter::api::build_router;
use tiltak_kafka_adapter::config::read_otel_tracing_config;
use tiltak_kafka_adapter::server::{
    async_task_handler, shutdown_handler, shutdown_signal_task, web_server_task,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    register_panic_logger();

    let otel_tracing_config = read_otel_tracing_config()?;

    setup_otel(otel_tracing_config)?;

    let app_state = Arc::new(simple_app_state::AppState::new());

    let router = build_router(app_state.clone());
    let server_task = web_server_task(router).await;

    let signal_task = shutdown_signal_task();

    app_state.set_has_started(true);

    tokio::select! {
        result = server_task => async_task_handler("Webserver", result),
        signal = signal_task => shutdown_handler(signal),
    }?;

    Ok(())
}
