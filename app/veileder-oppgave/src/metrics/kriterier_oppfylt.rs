use crate::domain::oppgave_type::OppgaveType;
use prometheus::{register_counter_vec, CounterVec};
use std::sync::LazyLock;
use strum::IntoEnumIterator;

static KRITERIER_OPPFYLT: LazyLock<CounterVec> = LazyLock::new(|| {
    let counter = register_counter_vec!(
        "veileder_oppgave_kriterier_oppfylt_total",
        "Antall hendelser som oppfyller oppgave-kriteriene per type (inkluderer duplikater)",
        &["type"]
    )
    .expect("Failed to register veileder_oppgave_kriterier_oppfylt_total counter");
    for oppgave_type in OppgaveType::iter() {
        counter.with_label_values(&[&oppgave_type.to_string()]);
    }
    counter
});

pub fn init() {
    LazyLock::force(&KRITERIER_OPPFYLT);
}

pub fn inkrement(oppgave_type: OppgaveType) {
    KRITERIER_OPPFYLT
        .with_label_values(&[&oppgave_type.to_string()])
        .inc();
}
