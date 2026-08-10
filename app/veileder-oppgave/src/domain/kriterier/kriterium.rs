use crate::domain::oppgave_type::OppgaveType;
use interne_hendelser::Hendelse;

pub struct Kriterium<H> {
    pub navn: &'static str,
    pub sjekk: fn(&H) -> bool,
}

/// ```compile_fail
/// use interne_hendelser::Avvist;
/// use veileder_oppgave::domain::kriterier::kriterium::OppgaveKriterier;
/// use veileder_oppgave::domain::oppgave_type::OppgaveType;
///
/// let _ = OppgaveKriterier::<Avvist>::new(OppgaveType::AvvistUnder18, &[]);
/// ```
pub struct OppgaveKriterier<H: Hendelse + 'static> {
    pub oppgave_type: OppgaveType,
    første: Kriterium<H>,
    resterende: &'static [Kriterium<H>],
}

impl<H: Hendelse + 'static> OppgaveKriterier<H> {
    pub const fn new(
        oppgave_type: OppgaveType,
        første: Kriterium<H>,
        resterende: &'static [Kriterium<H>],
    ) -> Self {
        Self {
            oppgave_type,
            første,
            resterende,
        }
    }

    pub fn oppfylt_av(&self, hendelse: &H) -> bool {
        self.oppfylt(&self.første, hendelse)
            && self
                .resterende
                .iter()
                .all(|kriterium| self.oppfylt(kriterium, hendelse))
    }

    fn oppfylt(&self, kriterium: &Kriterium<H>, hendelse: &H) -> bool {
        let oppfylt = (kriterium.sjekk)(hendelse);
        if !oppfylt {
            tracing::debug!(
                hendelse_id = %hendelse.hendelse_id(),
                kriterium = kriterium.navn,
                "kriterie ikke oppfylt"
            );
        }
        oppfylt
    }

    pub fn ikke_oppfylt_av(&self, hendelse: &H) -> bool {
        !self.oppfylt_av(hendelse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interne_hendelser::Avvist;
    use paw_test::hendelse_builder::AvvistBuilder;

    const TO_KRITERIER: OppgaveKriterier<Avvist> = OppgaveKriterier::new(
        OppgaveType::AvvistUnder18,
        Kriterium {
            navn: "alltid_sann",
            sjekk: |_| true,
        },
        &[Kriterium {
            navn: "har_arbeidssoeker_id_42",
            sjekk: |hendelse| hendelse.id == 42,
        }],
    );


    #[test]
    fn kan_opprette_med_kun_ett_kriterium() {
        let et_kriterie = OppgaveKriterier::new(
            OppgaveType::AvvistUnder18,
            Kriterium {
                navn: "alltid_sann",
                sjekk: |_| true,
            },
            &[],
        );
        assert!(et_kriterie.oppfylt_av(&AvvistBuilder::default().build()));
    }

    #[test]
    fn alle_oppfylt_gir_true() {
        assert!(
            TO_KRITERIER.oppfylt_av(
                &AvvistBuilder {
                    arbeidssoeker_id: 42,
                    ..Default::default()
                }
                .build()
            )
        );
    }

    #[test]
    fn ett_kriterium_ikke_oppfylt_gir_false() {
        assert!(
            !TO_KRITERIER.oppfylt_av(
                &AvvistBuilder {
                    arbeidssoeker_id: 99,
                    ..Default::default()
                }
                .build()
            )
        );
    }
}
