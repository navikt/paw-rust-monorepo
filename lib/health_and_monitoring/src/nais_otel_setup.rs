use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;
use opentelemetry::{global, KeyValue};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use tracing::log::info;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use paw_rust_base::env_var::get_env;
use paw_rust_base::error_handling::AppError;
use paw_rust_base::{nais_namespace, nais_otel_service_name};
use crate::otel_json_format_layer;

pub struct OtelSetupError {
    details: String,
}

impl Error for OtelSetupError {}

impl Debug for OtelSetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} ({:?})", &self.error_name(), &self.details))
    }
}

impl Display for OtelSetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Failed to setup otel environment: {}", &self.details))
    }
}

impl AppError for OtelSetupError {
    fn error_name(&self) -> &'static str {
        "OtelSetupError"
    }
}

pub fn nais_otlp_exporter() -> Result<Option<SpanExporter>, OtelSetupError> {
    let otel_endpoint = get_env("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    if let Some(otel_endpoint) = otel_endpoint {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_protocol(Protocol::Grpc)
            .with_endpoint(otel_endpoint)
            .with_timeout(Duration::from_secs(5))
            .build();
        exporter.map_err(|err| OtelSetupError {
            details: format!("Failed to create OTLP exporter: {}", err),
        }).map(Some)
    } else {
        Ok(None)
    }
}

pub fn setup_nais_otel() -> Result<(), OtelSetupError> {
    let exporter = nais_otlp_exporter()?;
    let exporter_active = exporter.is_some();
    let service_name = nais_otel_service_name().unwrap_or("local-build".to_string());
    let service_namespace = nais_namespace().unwrap_or("local".to_string());
    let builder = opentelemetry_sdk::trace::SdkTracerProvider::builder();
    let builder = if let Some(otlp_exporter) = exporter {
        builder.with_batch_exporter(otlp_exporter)
    } else {
        builder
    };
    let tracer_provider = builder.with_resource(
            Resource::builder_empty()
                .with_attributes([
                    KeyValue::new("service.name", service_name.clone()),
                    KeyValue::new("service.namespace", service_namespace.clone()),
                ])
                .build(),
        )
        .build();
    global::set_text_map_propagator(TraceContextPropagator::new());

    let fmt_layer = fmt::layer()
        .event_format(otel_json_format_layer::OtelJsonFormat)
        .with_ansi(false);
    let tracer = tracer_provider.tracer(service_name.clone());
    global::set_tracer_provider(tracer_provider);
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(OpenTelemetryLayer::new(tracer))
        .with(fmt_layer)
        .init();
    info!("Initialized NAIS OpenTelemetry with service name: {}, namespace: {}, exporter_active={}", service_name, service_namespace, exporter_active);
    Ok(())
}