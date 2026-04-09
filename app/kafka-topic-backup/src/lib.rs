pub mod config;
pub mod config_utils;
pub mod database;
pub mod errors;
pub mod kafka;
pub mod metrics;

pub const HWM_VERSION: i16 = 1;

// Re-export the functions we want to test from their proper location
pub use kafka::message_processor::{KafkaMessage, prosesser_melding};
