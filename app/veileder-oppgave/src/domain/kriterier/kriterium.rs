use crate::domain::oppgave_type::OppgaveType;
use interne_hendelser::Hendelse;

pub struct Kriterium<H> {
    pub navn: &'static str,
    pub sjekk: fn(&H) -> bool,
}

pub struct OppgaveKriterier<H: Hendelse + 'static> {
    pub oppgave_type: OppgaveType,
    kriterier: &'static [Kriterium<H>],
}

impl<H: Hendelse + 'static> OppgaveKriterier<H> {
    pub const fn new(
        oppgave_type: OppgaveType,
        kriterier: &'static [Kriterium<H>],
    ) -> Self {
        assert!(
            !kriterier.is_empty(),
            "OppgaveKriterier må inneholde minst ett kriterium"
        );
        Self { oppgave_type, kriterier }
    }

    pub fn oppfylt_av(&self, hendelse: &H) -> bool {
        self.kriterier.iter().all(|kriterium| {
            let oppfylt = (kriterium.sjekk)(hendelse);
            if !oppfylt {
                tracing::debug!(
                    hendelse_id = %hendelse.hendelse_id(),
                    kriterium = kriterium.navn,
                    "kriterie ikke oppfylt"
                );
            }
            oppfylt
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interne_hendelser::Avvist;
    use paw_test::hendelse_builder::AvvistBuilder;

    const TO_KRITERIER: OppgaveKriterier<Avvist> = OppgaveKriterier::new(
        OppgaveType::AvvistUnder18,
        &[
            Kriterium {
                navn: "alltid_sann",
                sjekk: |_| true,
            },
            Kriterium {
                navn: "har_arbeidssoeker_id_42",
                sjekk: |hendelse| hendelse.id == 42,
            },
        ],
    );

    #[test]
    #[should_panic(expected = "OppgaveKriterier må inneholde minst ett kriterium")]
    fn tom_liste_panicer() {
        OppgaveKriterier::<Avvist>::new(OppgaveType::AvvistUnder18, &[]);
    }

    #[test]
    fn alle_oppfylt_gir_true() {
        assert!(TO_KRITERIER.oppfylt_av(
            &AvvistBuilder {
                arbeidssoeker_id: 42,
                ..Default::default()
            }
            .build()
        ));
    }

    #[test]
    fn ett_kriterium_ikke_oppfylt_gir_false() {
        assert!(!TO_KRITERIER.oppfylt_av(
            &AvvistBuilder {
                arbeidssoeker_id: 99,
                ..Default::default()
            }
            .build()
        ));
    }
}
