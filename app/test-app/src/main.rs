mod app_logic;
mod http_apis;
mod logging;

use crate::http_apis::register_http_apis;
use crate::logging::init_log;
use health::simple_app_state::AppState;
use log::{info, log};
use opentelemetry::{global, trace::Tracer, KeyValue};
use opentelemetry_otlp::{Protocol, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use std::sync::Arc;
use std::time::Duration;
use futures::SinkExt;
use tonic::metadata::*;
use tracing::instrument;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*};

#[tokio::main]
async fn main() {
    //init_log();

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_protocol(Protocol::Grpc)
        .with_endpoint(std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap())
        .with_timeout(Duration::from_secs(5))
        .build().unwrap();

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap();
    let service_namespace = std::env::var("NAIS_NAMESPACE").unwrap();
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(Resource::builder_empty().with_attributes(
            [
                KeyValue::new("service.name", service_name.clone()),
                KeyValue::new("service.namespace", service_namespace.clone()),
            ])
            .build())
        .build();
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    let fmt_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_level(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);
    global::set_tracer_provider(tracer_provider);
    let tracer = global::tracer(service_name.clone());
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with(fmt_layer)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
    info!("Starter test app");
    info!("Service Name: {}, Namespace: {}", service_name, service_namespace);

    test_trace();
    match run_app().await {
        Ok(()) => println!("Application exited successfully."),
        Err(e) => eprintln!("Application error (code {}): {}", e.code(), e.description()),
    }
}

#[instrument]
fn test_trace() {
    info!("Kjørte metode med 'instrument'");
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
