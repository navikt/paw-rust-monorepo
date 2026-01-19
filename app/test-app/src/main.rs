mod app_logic;
mod http_apis;
mod logging;

use crate::http_apis::register_http_apis;
use health::simple_app_state::AppState;
use opentelemetry::{global, trace::Tracer};
use std::sync::Arc;
use log::{info, log};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_otlp::{WithExportConfig, Protocol};
use crate::logging::init_log;

#[tokio::main]
async fn main() {
    init_log();
    info!("Starter test app");
    global::set_text_map_propagator(TraceContextPropagator::new());
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_protocol(Protocol::Grpc)
        .build().unwrap();


    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .build();

    global::set_tracer_provider(tracer_provider);

    match run_app().await {
        Ok(()) => println!("Application exited successfully."),
        Err(e) => eprintln!("Application error (code {}): {}", e.code(), e.description()),
    }
}

async fn run_app() -> Result<(), Box<dyn AppError>> {
    let app_state = Arc::new(AppState::new());
    let app_logic = Arc::new(app_logic::AppLogic::new(Arc::from("Hello")));
    let http_server_task = register_http_apis(app_state.clone(), app_logic.clone());
    app_state.set_has_started(true);
    match http_server_task.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(Box::new(GenericError {
            description: format!("HTTP server error: {}", e),
            code: 500,
        })),
        Err(e) => Err(Box::new(GenericError {
            description: format!("Task join error: {}", e),
            code: 500,
        })),
    }
}

trait AppError {
    fn description(&self) -> &str;
    fn code(&self) -> u16;
}

struct GenericError {
    description: String,
    code: u16,
}

impl AppError for GenericError {
    fn description(&self) -> &str {
        &self.description
    }

    fn code(&self) -> u16 {
        self.code
    }
}
