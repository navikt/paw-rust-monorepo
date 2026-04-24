use crate::metrics::ekstern_oppgave_opprettelse_feil;
use crate::metrics::kriterier_oppfylt;

pub fn init() {
    ekstern_oppgave_opprettelse_feil::init();
    kriterier_oppfylt::init();
}
