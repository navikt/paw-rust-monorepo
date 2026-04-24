use crate::client::oppgave_client::{OppgaveApiError, OppgaveApiErrorDiscriminants};
use prometheus::{register_counter_vec, CounterVec};
use std::sync::LazyLock;
use strum::IntoEnumIterator;

static EKSTERN_OPPRETTELSE_FEILET: LazyLock<CounterVec> = LazyLock::new(|| {
    let counter = register_counter_vec!(
        "veileder_oppgave_ekstern_oppgave_opprettelse_feil_total",
        "Antall feil ved opprettelse av oppgave mot eksternt Oppgave API",
        &["feil_type"]
    )
    .expect("Failed to register veileder_oppgave_ekstern_oppgave_opprettelse_feil_total counter");
    for variant in OppgaveApiErrorDiscriminants::iter() {
        counter.with_label_values(&[&variant.to_string()]);
    }
    counter
});

pub fn init() {
    LazyLock::force(&EKSTERN_OPPRETTELSE_FEILET);
}

pub fn inkrement_ekstern_oppgave_opprettelse_feil(error: &OppgaveApiError) {
    let feil_type = OppgaveApiErrorDiscriminants::from(error).to_string();
    EKSTERN_OPPRETTELSE_FEILET
        .with_label_values(&[&feil_type])
        .inc();
}
