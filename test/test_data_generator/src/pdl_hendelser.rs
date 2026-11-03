use chrono::Utc;
use pdl_hendelser::leesah::opplysninger::navn::Navn;
use pdl_hendelser::leesah::{Endringstype, Opplysningstype, Personhendelse};
use uuid::Uuid;

pub fn create_dummy_personhendelse(
    personidenter: Vec<&str>,
    opplysningstype: Opplysningstype,
    endringstype: Endringstype,
    navn: Option<Navn>,
) -> Personhendelse {
    Personhendelse {
        hendelse_id: Uuid::new_v4().to_string(),
        personidenter: personidenter.iter().map(|&s| s.to_string()).collect(),
        master: "FOLKEREGISTERET".to_string(),
        opprettet: Utc::now(),
        opplysningstype,
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

pub fn create_dummy_personhendelse_navn(
    fornavn: &str,
    mellomnavn: Option<&str>,
    etternavn: &str,
) -> Navn {
    Navn {
        fornavn: fornavn.to_string(),
        mellomnavn: mellomnavn.map(|s| s.to_string()),
        etternavn: etternavn.to_string(),
        forkortet_navn: None,
        originalt_navn: None,
        gyldig_fra_og_med: None,
    }
}
