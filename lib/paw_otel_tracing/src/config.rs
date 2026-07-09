use crate::otel_format::OtelFormat;
use crate::serde::deserialize_tracing_level;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OtelTracingConfig {
    #[serde(deserialize_with = "deserialize_tracing_level")]
    pub level: tracing::Level,
    pub format: OtelFormat,
}
