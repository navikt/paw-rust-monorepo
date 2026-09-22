use crate::error::OtelError;
use crate::otel_json_format_layer;
use anyhow::Result;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use paw_rust_base::env::{get_env, nais_namespace, nais_otel_service_name};
use std::time::Duration;
use tracing::info;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

pub fn nais_otlp_exporter() -> Result<Option<SpanExporter>> {
    let otel_endpoint = get_env("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    if let Some(otel_endpoint) = otel_endpoint {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_protocol(Protocol::Grpc)
            .with_endpoint(otel_endpoint)
            .with_timeout(Duration::from_secs(5))
            .build()
            .map_err(OtelError::CreateOtlpExporter)?;
        Ok(Some(exporter))
    } else {
        Ok(None)
    }
}

pub fn setup_nais_otel() -> Result<()> {
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

    let fmt_layer = fmt::layer()
        .event_format(otel_json_format_layer::OtelJsonFormat)
        .with_ansi(false);
    let tracer = tracer_provider.tracer(service_name.clone());
    global::set_tracer_provider(tracer_provider);
    tracing_subscriber::registry()
        .with(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into())
                .add_directive("sqlx::query=info".parse()?)
                // h2/tonic/hyper/rustls bærer OTLP-gRPC-transporten for span-eksport.
                // På DEBUG-nivå logger h2 én terse linje ("received"/"send") per
                // HTTP/2-frame, som drukner ut nyttige applikasjonslogger.
                .add_directive("h2=info".parse()?)
                .add_directive("tonic=info".parse()?)
                .add_directive("tower=info".parse()?)
                .add_directive("hyper=info".parse()?)
                .add_directive("hyper_util=info".parse()?)
                .add_directive("rustls=info".parse()?)
                // opentelemetry sine interne otel_debug!/otel_info!-kall (aktivert av
                // "internal-logs", som er default-feature) har ikke noe "message"-felt,
                // kun et strukturert "name"-felt. På DEBUG-niva ser disse ut som tomme
                // logglinjer i Grafana/Loki, som viser "message" som forhandsvisning.
                .add_directive("opentelemetry=info".parse()?)
                .add_directive("opentelemetry_sdk=info".parse()?)
                .add_directive("opentelemetry-otlp=info".parse()?),
        )
        .with(OpenTelemetryLayer::new(tracer))
        .with(fmt_layer)
        .init();
    info!(
        "Initialized NAIS OpenTelemetry with service name: {}, namespace: {}, exporter_active={}",
        service_name, service_namespace, exporter_active
    );
    Ok(())
}
