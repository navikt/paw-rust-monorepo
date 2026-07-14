use chrono::Utc;
use dab_oppfolgingperioder::kontor::Kontor;
use dab_oppfolgingperioder::oppfolgingsperiode::{Oppfolgingsperiode, OppfolgingsperiodeEndret};
use uuid::Uuid;

pub fn create_dummy_startet_oppfolgingsperiode<'a>(
    oppfolgingsperiode_id: Uuid,
    aktor_id: &'a str,
    identitetsnummer: &'a str,
    kontor_id: &'a str,
) -> Oppfolgingsperiode {
    Oppfolgingsperiode::Startet(OppfolgingsperiodeEndret {
        oppfolgingsperiode_id,
        aktor_id: aktor_id.to_string(),
        ident: identitetsnummer.to_string(),
        kontor: Kontor {
            kontor_id: kontor_id.to_string(),
            kontor_navn: format!("Kontor {}", kontor_id),
        },
        start_tidspunkt: Utc::now(),
    })
}
