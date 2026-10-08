pub mod config;
pub mod error;
mod exporter;
pub mod format;
mod json_format;
mod nais_otel_setup;
mod otel_setup;

pub use nais_otel_setup::setup_nais_otel;
pub use otel_setup::setup_otel;
