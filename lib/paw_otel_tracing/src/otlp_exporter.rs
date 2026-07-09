use crate::error::OtelError;
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use paw_rust_base::env::get_env;
use std::time::Duration;

pub fn nais_otlp_exporter() -> anyhow::Result<Option<SpanExporter>> {
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
