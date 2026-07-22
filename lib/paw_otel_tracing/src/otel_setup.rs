use crate::config::OtelTracingConfig;
use crate::otlp_exporter::nais_otlp_exporter;
use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use paw_rust_base::env::{nais_namespace, nais_otel_service_name};
use tracing::info;
use tracing_opentelemetry::OpenTelemetryLayer;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

pub fn setup_otel(config: OtelTracingConfig) -> Result<()> {
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
    let tracer_provider = builder
        .with_resource(
            Resource::builder_empty()
                .with_attributes([
                    KeyValue::new("service.name", service_name.clone()),
                    KeyValue::new("service.namespace", service_namespace.clone()),
                ])
                .build(),
        )
        .build();
    global::set_text_map_propagator(TraceContextPropagator::new());

    let tracer = tracer_provider.tracer(service_name.clone());
    global::set_tracer_provider(tracer_provider);

    let filter = config
        .directives
        .iter()
        .try_fold(EnvFilter::from_default_env(), |filter, directive| {
            directive.parse().map(|d| filter.add_directive(d))
        })?;

    let fmt_layer = fmt::layer().event_format(config.format).with_ansi(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(OpenTelemetryLayer::new(tracer))
        .with(fmt_layer)
        .init();
    info!(
        "Initialized NAIS OpenTelemetry with service name: {}, namespace: {}, exporter_active={}",
        service_name, service_namespace, exporter_active
    );
    Ok(())
}
