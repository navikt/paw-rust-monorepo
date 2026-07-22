use crate::otel_format::OtelFormat;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OtelTracingConfig {
    pub format: OtelFormat,
    pub directives: Vec<String>,
}
