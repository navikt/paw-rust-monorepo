pub mod config;
pub mod database;
pub mod kafka;
pub mod metrics;

pub use kafka::message_processor::{KafkaMessage, prosesser_melding};

