pub mod consumer_function;
pub mod kontroler_pdl_info;
pub mod dao;
pub mod domain;
pub mod kafka;
pub mod oppdater_pdl_data;
pub mod pdl;
pub mod pdl_oppdatering_task;

pub const HENDELSELOGG_TOPIC: &str = "paw.arbeidssoker-hendelseslogg-v1";
pub const ARBEIDSSOKERPERIODER_TOPIC: &str = "paw.arbeidssokerperioder-v1";
