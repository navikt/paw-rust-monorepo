use chrono::{DateTime, Utc};
use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
use eksterne_hendelser::bekreftelse::paa_vegne_av::{Handling, PaaVegneAv};
use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
use eksterne_hendelser::bekreftelse::vo::start::Start;
use eksterne_hendelser::bekreftelse::vo::stopp::Stopp;
use eksterne_hendelser::bekreftelse::vo::svar::Svar;
use eksterne_hendelser::egenvurdering::Egenvurdering;
use eksterne_hendelser::opplysninger::Opplysninger;
use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::profilering::Profilering;
use eksterne_hendelser::vo::annet::Annet;
use eksterne_hendelser::vo::bruker::Bruker;
use eksterne_hendelser::vo::brukertype::BrukerType;
use eksterne_hendelser::vo::helse::Helse;
use eksterne_hendelser::vo::ja_nei_vet_ikke::JaNeiVetIkke;
use eksterne_hendelser::vo::jobbsituasjon::{Beskrivelse, BeskrivelseMedDetaljer, Jobbsituasjon};
use eksterne_hendelser::vo::metadata::Metadata;
use eksterne_hendelser::vo::profilert_til::ProfilertTil;
use eksterne_hendelser::vo::utdanning::Utdanning;
use std::collections::HashMap;
use uuid::Uuid;

pub fn create_dummy_startet_periode(periode_id: Uuid) -> Periode {
    let identitetsnummer = "01017012345";
    Periode {
        id: periode_id,
        identitetsnummer: identitetsnummer.to_string(),
        startet: create_dummy_metadata(identitetsnummer.to_string()),
        avsluttet: None,
    }
}

pub fn create_dummy_avsluttet_periode(periode_id: Uuid) -> Periode {
    let identitetsnummer = "01017012345";
    Periode {
        id: periode_id,
        identitetsnummer: "01017012345".to_string(),
        startet: create_dummy_metadata(identitetsnummer.to_string()),
        avsluttet: Some(create_dummy_metadata(identitetsnummer.to_string())),
    }
}

pub fn create_dummy_opplysninger(periode_id: Uuid, opplysninger_id: Uuid) -> Opplysninger {
    let identitetsnummer = "01017012345";
    Opplysninger {
        id: opplysninger_id,
        periode_id,
        sendt_inn_av: create_dummy_metadata(identitetsnummer.to_string()),
        utdanning: Some(Utdanning {
            nus: "1234".to_string(),
            bestaatt: Some(JaNeiVetIkke::Ja),
            godkjent: Some(JaNeiVetIkke::Ja),
        }),
        helse: Some(Helse {
            helsetilstand_hindrer_arbeid: JaNeiVetIkke::Nei,
        }),
        jobbsituasjon: Jobbsituasjon {
            beskrivelser: vec![BeskrivelseMedDetaljer {
                beskrivelse: Beskrivelse::HarBlittSagtOpp,
                detaljer: HashMap::from([
                    ("oppsigelsesdato".to_string(), "2024-06-30".to_string()),
                    ("arbeidsgiver".to_string(), "Test AS".to_string()),
                ]),
            }],
        },
        annet: Some(Annet {
            andre_forhold_hindrer_arbeid: Some(JaNeiVetIkke::Nei),
        }),
    }
}

pub fn create_dummy_profilering(
    periode_id: Uuid,
    opplysninger_id: Uuid,
    profilering_id: Uuid,
) -> Profilering {
    let identitetsnummer = "01017012345";
    Profilering {
        id: profilering_id,
        periode_id,
        opplysninger_om_arbeidssoker_id: opplysninger_id,
        sendt_inn_av: create_dummy_metadata(identitetsnummer.to_string()),
        profilert_til: ProfilertTil::AntattGodeMuligheter,
        jobbet_sammenhengende_seks_av_tolv_siste_mnd: false,
        alder: Some(42),
    }
}

pub fn create_dummy_egenvurdering(
    periode_id: Uuid,
    profilering_id: Uuid,
    egenvurdering_id: Uuid,
) -> Egenvurdering {
    let identitetsnummer = "01017012345";
    Egenvurdering {
        id: egenvurdering_id,
        periode_id,
        profilering_id,
        sendt_inn_av: create_dummy_metadata(identitetsnummer.to_string()),
        profilert_til: ProfilertTil::AntattGodeMuligheter,
        egenvurdering: ProfilertTil::OppgittHindringer,
    }
}

pub fn create_dummy_bekreftelse(periode_id: Uuid, bekreftelse_id: Uuid) -> Bekreftelse {
    Bekreftelse {
        id: bekreftelse_id,
        periode_id,
        bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
        svar: create_dummy_svar(),
    }
}

pub fn create_dummy_svar() -> Svar {
    let identitetsnummer = "01017012345";
    Svar {
        sendt_inn_av: create_dummy_metadata(identitetsnummer.to_string()),
        gjelder_fra: datetime_rfc3339("2026-06-16T12:00:00Z"),
        gjelder_til: datetime_rfc3339("2026-06-30T12:00:00Z"),
        har_jobbet_i_denne_perioden: false,
        vil_fortsette_som_arbeidssoeker: true,
    }
}

pub fn create_dummy_paavegneav_start(periode_id: Uuid) -> PaaVegneAv {
    PaaVegneAv {
        periode_id,
        bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
        handling: Handling::Start(Start {
            interval_ms: 5,
            grace_ms: 3,
        }),
    }
}

pub fn create_dummy_paavegneav_stopp(periode_id: Uuid) -> PaaVegneAv {
    PaaVegneAv {
        periode_id,
        bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
        handling: Handling::Stopp(Stopp { frist_brutt: true }),
    }
}

pub fn create_dummy_metadata(identitetsnummer: String) -> Metadata {
    Metadata {
        tidspunkt: datetime_rfc3339("2026-06-30T12:00:00Z"),
        utfoert_av: Bruker {
            bruker_type: BrukerType::Sluttbruker,
            id: identitetsnummer,
            sikkerhetsnivaa: Some("tokenx:Level4".to_string()),
        },
        kilde: "test-system".to_string(),
        aarsak: "Test".to_string(),
        tidspunkt_fra_kilde: None,
    }
}

fn datetime_rfc3339(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
        .unwrap()
        .with_timezone(&Utc)
}
