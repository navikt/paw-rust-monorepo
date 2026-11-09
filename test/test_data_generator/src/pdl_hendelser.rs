use chrono::Utc;
use pdl_hendelser::leesah::opplysninger::bostedsadresse::Bostedsadresse;
use pdl_hendelser::leesah::opplysninger::common::adresse::{
    Koordinater, Matrikkeladresse, Vegadresse,
};
use pdl_hendelser::leesah::opplysninger::navn::Navn;
use pdl_hendelser::leesah::{Endringstype, Opplysningstype, Personhendelse};
use uuid::Uuid;

pub fn create_dummy_personhendelse_navn(
    personidenter: Vec<&str>,
    endringstype: Endringstype,
    navn: Option<Navn>,
) -> Personhendelse {
    Personhendelse {
        hendelse_id: Uuid::new_v4().to_string(),
        personidenter: personidenter.iter().map(|&s| s.to_string()).collect(),
        master: "FOLKEREGISTERET".to_string(),
        opprettet: Utc::now(),
        opplysningstype: Opplysningstype::NavnV1,
        endringstype,
        tidligere_hendelse_id: None,
        adressebeskyttelse: None,
        delt_bosted: None,
        doedfoedt_barn: None,
        doedsfall: None,
        falsk_identitet: None,
        foedsel: None,
        foedselsdato: None,
        forelder_barn_relasjon: None,
        familierelasjon: None,
        sivilstand: None,
        vergemaal_eller_fremtidsfullmakt: None,
        utflytting_fra_norge: None,
        innflytting_til_norge: None,
        folkeregisteridentifikator: None,
        navn,
        sikkerhetstiltak: None,
        statsborgerskap: None,
        telefonnummer: None,
        kontaktadresse: None,
        bostedsadresse: None,
    }
}

pub fn create_dummy_personhendelse_bostedsadresse(
    personidenter: Vec<&str>,
    endringstype: Endringstype,
    bostedsadresse: Option<Bostedsadresse>,
) -> Personhendelse {
    Personhendelse {
        hendelse_id: Uuid::new_v4().to_string(),
        personidenter: personidenter.iter().map(|&s| s.to_string()).collect(),
        master: "FOLKEREGISTERET".to_string(),
        opprettet: Utc::now(),
        opplysningstype: Opplysningstype::BostedsadresseV1,
        endringstype,
        tidligere_hendelse_id: None,
        adressebeskyttelse: None,
        delt_bosted: None,
        doedfoedt_barn: None,
        doedsfall: None,
        falsk_identitet: None,
        foedsel: None,
        foedselsdato: None,
        forelder_barn_relasjon: None,
        familierelasjon: None,
        sivilstand: None,
        vergemaal_eller_fremtidsfullmakt: None,
        utflytting_fra_norge: None,
        innflytting_til_norge: None,
        folkeregisteridentifikator: None,
        navn: None,
        sikkerhetstiltak: None,
        statsborgerskap: None,
        telefonnummer: None,
        kontaktadresse: None,
        bostedsadresse,
    }
}

pub fn create_dummy_navn(fornavn: &str, mellomnavn: Option<&str>, etternavn: &str) -> Navn {
    Navn {
        fornavn: fornavn.to_string(),
        mellomnavn: mellomnavn.map(|s| s.to_string()),
        etternavn: etternavn.to_string(),
        forkortet_navn: None,
        originalt_navn: None,
        gyldig_fra_og_med: None,
    }
}

pub fn create_dummy_bostedsadresse() -> Bostedsadresse {
    Bostedsadresse {
        angitt_flyttedato: None,
        gyldig_fra_og_med: None,
        gyldig_til_og_med: None,
        co_adressenavn: None,
        vegadresse: Some(create_dummy_vegadresse()),
        matrikkeladresse: Some(create_dummy_matrikkeladresse()),
        utenlandsk_adresse: None,
        ukjent_bosted: None,
    }
}

fn create_dummy_vegadresse() -> Vegadresse {
    Vegadresse {
        matrikkel_id: to_opt_string("1"),
        husnummer: to_opt_string("2"),
        husbokstav: to_opt_string("B"),
        bruksenhetsnummer: to_opt_string("H0101"),
        adressenavn: to_opt_string("Gate"),
        kommunenummer: to_opt_string("0301"),
        bydelsnummer: to_opt_string("030101"),
        tilleggsnavn: to_opt_string("Tillegg"),
        postnummer: to_opt_string("0150"),
        koordinater: Some(Koordinater {
            x: Some(1.5),
            y: Some(2.5),
            z: None,
        }),
    }
}

fn create_dummy_matrikkeladresse() -> Matrikkeladresse {
    Matrikkeladresse {
        matrikkel_id: to_opt_string("1"),
        bruksenhetsnummer: to_opt_string("H0101"),
        tilleggsnavn: to_opt_string("Gård"),
        postnummer: to_opt_string("0150"),
        kommunenummer: to_opt_string("0301"),
        koordinater: Some(Koordinater {
            x: Some(1.0),
            y: None,
            z: Some(3.0),
        }),
    }
}

fn to_opt_string(v: &str) -> Option<String> {
    Some(v.to_string())
}
