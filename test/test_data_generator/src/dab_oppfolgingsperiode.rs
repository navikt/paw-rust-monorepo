use chrono::Utc;
use dab_oppfolgingperioder::kontor::Kontor;
use dab_oppfolgingperioder::oppfolgingsperiode::{
    Oppfolgingsperiode, OppfolgingsperiodeAvsluttet, OppfolgingsperiodeEndret,
};
use uuid::Uuid;

pub fn create_dummy_oppfolgingsperiode_startet<'a>(
    oppfolgingsperiode_id: Uuid,
    aktor_id: &'a str,
    identitetsnummer: &'a str,
    kontor_id: &'a str,
) -> Oppfolgingsperiode {
    Oppfolgingsperiode::Startet(oppfolgingsperiode_endret(
        oppfolgingsperiode_id,
        aktor_id,
        identitetsnummer,
        kontor_id,
    ))
}

pub fn create_dummy_oppfolgingsperiode_endret<'a>(
    oppfolgingsperiode_id: Uuid,
    aktor_id: &'a str,
    identitetsnummer: &'a str,
    kontor_id: &'a str,
) -> Oppfolgingsperiode {
    Oppfolgingsperiode::Endret(oppfolgingsperiode_endret(
        oppfolgingsperiode_id,
        aktor_id,
        identitetsnummer,
        kontor_id,
    ))
}

pub fn create_dummy_oppfolgingsperiode_avsluttet<'a>(
    oppfolgingsperiode_id: Uuid,
    aktor_id: &'a str,
    identitetsnummer: &'a str,
) -> Oppfolgingsperiode {
    Oppfolgingsperiode::Avsluttet(OppfolgingsperiodeAvsluttet {
        id: oppfolgingsperiode_id,
        aktor_id: aktor_id.to_string(),
        ident: identitetsnummer.to_string(),
        start_tidspunkt: Utc::now(),
        slutt_tidspunkt: Utc::now(),
    })
}

fn oppfolgingsperiode_endret<'a>(
    oppfolgingsperiode_id: Uuid,
    aktor_id: &'a str,
    identitetsnummer: &'a str,
    kontor_id: &'a str,
) -> OppfolgingsperiodeEndret {
    OppfolgingsperiodeEndret {
        id: oppfolgingsperiode_id,
        aktor_id: aktor_id.to_string(),
        ident: identitetsnummer.to_string(),
        kontor: Kontor {
            kontor_id: kontor_id.to_string(),
            kontor_navn: format!("Kontor {}", kontor_id),
        },
        start_tidspunkt: Utc::now(),
    }
}
