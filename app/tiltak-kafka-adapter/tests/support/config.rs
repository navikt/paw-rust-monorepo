use std::time::Duration;
use tiltak_kafka_adapter::config::SynkConfig;

pub fn synk_config(maks_feil_per_kjoering: u32) -> SynkConfig {
    SynkConfig {
        aktivert: true,
        task_interval: Duration::from_mins(10),
        synk_interval: Duration::from_hours(2),
        batch_size: 100,
        batch_grace_time: Duration::ZERO,
        request_grace_time: Duration::ZERO,
        maks_feil_per_rad: 10,
        maks_feil_per_kjoering,
    }
}
