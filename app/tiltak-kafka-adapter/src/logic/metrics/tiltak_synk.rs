use prometheus::{IntCounterVec, register_int_counter_vec};
use std::sync::LazyLock;

static TILTAKSHISTORIKK_COUNTER: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "paw_tiltak_kafka_adapter_tiltakshistorikk_total",
        "Antall synkroniseringer av tiltakshistorikk per resultat",
        &["resultat"]
    )
    .expect("Failed to register paw_tiltak_kafka_adapter_tiltakshistorikk_total counter")
});

pub fn registrer_synkronisert() {
    TILTAKSHISTORIKK_COUNTER
        .with_label_values(&["synkronisert"])
        .inc();
}

pub fn registrer_feilet() {
    TILTAKSHISTORIKK_COUNTER
        .with_label_values(&["feilet"])
        .inc();
}

pub fn registrer_avbrutt(antall: u64) {
    TILTAKSHISTORIKK_COUNTER
        .with_label_values(&["avbrutt"])
        .inc_by(antall);
}
