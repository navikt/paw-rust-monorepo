use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OtelError {
    #[error("Failed to create OTLP exporter: {0}")]
    CreateOtlpExporter(opentelemetry_otlp::ExporterBuildError),
}
