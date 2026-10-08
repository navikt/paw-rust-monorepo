pub mod config;
pub mod error;
mod exporter;
pub mod format;
mod json_format;
mod otel_setup;

pub use config::default_config;
pub use otel_setup::{setup_nais_otel, setup_otel};
