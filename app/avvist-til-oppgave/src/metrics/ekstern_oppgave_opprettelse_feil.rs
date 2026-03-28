use crate::client::oppgave_client::{OppgaveApiError, OppgaveApiErrorDiscriminants};
use prometheus::{register_counter_vec, CounterVec};
use std::sync::OnceLock;
use strum::IntoEnumIterator;

static EKSTERN_OPPRETTELSE_FEILET: OnceLock<CounterVec> = OnceLock::new();

pub fn inkrement_ekstern_oppgave_opprettelse_feil(error: &OppgaveApiError) {
    let feil_type = OppgaveApiErrorDiscriminants::from(error).to_string();
    EKSTERN_OPPRETTELSE_FEILET
        .get_or_init(|| {
            let counter = register_counter_vec!(
                "avvist_til_oppgave_ekstern_oppgave_opprettelse_feil_total",
                "Antall feil ved opprettelse av oppgave mot eksternt Oppgave API",
                &["feil_type"]
            )
            .expect("Failed to register avvist_til_oppgave_ekstern_opprettelse_feilet_total counter");
            for variant in OppgaveApiErrorDiscriminants::iter() {
                counter.with_label_values(&[&variant.to_string()]);
            }
            counter
        })
        .with_label_values(&[&feil_type])
        .inc();
}
