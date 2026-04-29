use crate::metrics::ekstern_oppgave_opprettelse_feil;
use crate::metrics::kriterier_oppfylt;

pub fn init_metrics() {
    ekstern_oppgave_opprettelse_feil::init();
    kriterier_oppfylt::init();
}
