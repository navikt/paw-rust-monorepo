pub mod consumer_function;
pub mod dao;
pub mod kafka;
pub mod kontroll;
pub mod pdl;
pub mod pdl_oppdatering;

pub const HENDELSELOGG_TOPIC: &str = "paw.arbeidssoker-hendelseslogg-v1";
pub const ARBEIDSSOKERPERIODER_TOPIC: &str = "paw.arbeidssokerperioder-v1";
