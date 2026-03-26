use crate::metrics::db::OppgaveStatusAntall;
use crate::domain::oppgave_status::OppgaveStatus;
use prometheus::{GaugeVec, register_gauge_vec};
use std::sync::OnceLock;
use strum::IntoEnumIterator;

static OPPGAVER_PER_STATUS: OnceLock<GaugeVec> = OnceLock::new();

pub fn set_oppgave_status_counts(oppgave_status_antall: &[OppgaveStatusAntall]) {
    let gauge = oppgaver_per_status_gauge();

    for oppgave_status in OppgaveStatus::iter() {
        let antall = oppgave_status_antall
            .iter()
            .find(|entry| entry.oppgave_status == oppgave_status)
            .map(|entry| entry.antall)
            .unwrap_or(0);
        gauge
            .with_label_values(&[&oppgave_status.to_string()])
            .set(antall as f64);
    }
}

fn oppgaver_per_status_gauge() -> &'static GaugeVec {
    OPPGAVER_PER_STATUS.get_or_init(|| {
        register_gauge_vec!(
            "avvist_til_oppgave_oppgaver_total",
            "Antall oppgaver per status",
            &["status"]
        )
        .expect("Failed to register avvist_til_oppgave_oppgaver_total gauge")
    })
}
