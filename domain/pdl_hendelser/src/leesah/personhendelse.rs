use crate::leesah::opplysninger::adressebeskyttelse::Adressebeskyttelse;
use crate::leesah::opplysninger::bostedsadresse::Bostedsadresse;
use crate::leesah::opplysninger::delt_bosted::DeltBosted;
use crate::leesah::opplysninger::doedfoedt_barn::DoedfoedtBarn;
use crate::leesah::opplysninger::doedsfall::Doedsfall;
use crate::leesah::opplysninger::falsk_identitet::FalskIdentitet;
use crate::leesah::opplysninger::familierelasjon::Familierelasjon;
use crate::leesah::opplysninger::foedsel::Foedsel;
use crate::leesah::opplysninger::foedselsdato::Foedselsdato;
use crate::leesah::opplysninger::folkeregisteridentifikator::Folkeregisteridentifikator;
use crate::leesah::opplysninger::forelder_barn_relasjon::ForelderBarnRelasjon;
use crate::leesah::opplysninger::innflytting::InnflyttingTilNorge;
use crate::leesah::opplysninger::kontaktadresse::Kontaktadresse;
use crate::leesah::opplysninger::navn::Navn;
use crate::leesah::opplysninger::sikkerhetstiltak::Sikkerhetstiltak;
use crate::leesah::opplysninger::sivilstand::Sivilstand;
use crate::leesah::opplysninger::statsborgerskap::Statsborgerskap;
use crate::leesah::opplysninger::telefonnummer::Telefonnummer;
use crate::leesah::opplysninger::utflytting::UtflyttingFraNorge;
use crate::leesah::opplysninger::verge::VergemaalEllerFremtidsfullmakt;
use crate::leesah::{Endringstype, Opplysningstype};
use crate::serde::{AvroDatumDeserializer, AvroDatumSerializer, AvroSerdeError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampMilliSeconds, serde_as};
use std::fmt;

pub const PERSONHENDELSE_SCHEMA: &str = include_str!("../../avro/avsc/leesah/Personhendelse.avsc");

#[serde_as]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Personhendelse {
    pub hendelse_id: String,
    pub personidenter: Vec<String>,
    pub master: String,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub opprettet: DateTime<Utc>,
    pub opplysningstype: Opplysningstype,
    pub endringstype: Endringstype,
    pub tidligere_hendelse_id: Option<String>,
    pub adressebeskyttelse: Option<Adressebeskyttelse>,
    pub delt_bosted: Option<DeltBosted>,
    pub doedfoedt_barn: Option<DoedfoedtBarn>,
    pub doedsfall: Option<Doedsfall>,
    pub falsk_identitet: Option<FalskIdentitet>,
    pub foedsel: Option<Foedsel>,
    pub foedselsdato: Option<Foedselsdato>,
    pub forelder_barn_relasjon: Option<ForelderBarnRelasjon>,
    pub familierelasjon: Option<Familierelasjon>,
    pub sivilstand: Option<Sivilstand>,
    pub vergemaal_eller_fremtidsfullmakt: Option<VergemaalEllerFremtidsfullmakt>,
    pub utflytting_fra_norge: Option<UtflyttingFraNorge>,
    #[serde(rename = "InnflyttingTilNorge")]
    pub innflytting_til_norge: Option<InnflyttingTilNorge>,
    #[serde(rename = "Folkeregisteridentifikator")]
    pub folkeregisteridentifikator: Option<Folkeregisteridentifikator>,
    pub navn: Option<Navn>,
    pub sikkerhetstiltak: Option<Sikkerhetstiltak>,
    pub statsborgerskap: Option<Statsborgerskap>,
    pub telefonnummer: Option<Telefonnummer>,
    pub kontaktadresse: Option<Kontaktadresse>,
    pub bostedsadresse: Option<Bostedsadresse>,
}

impl fmt::Debug for Personhendelse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Personhendelse")
            .field("hendelse_id", &self.hendelse_id)
            .field("master", &self.master)
            .field("opprettet", &self.opprettet)
            .field("opplysningstype", &self.opplysningstype)
            .field("endringstype", &self.endringstype)
            .field("tidligere_hendelse_id", &self.tidligere_hendelse_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct PersonhendelseDeserializer(AvroDatumDeserializer);

impl PersonhendelseDeserializer {
    pub fn new() -> Result<Self, AvroSerdeError> {
        AvroDatumDeserializer::new(PERSONHENDELSE_SCHEMA).map(Self)
    }

    pub fn deserialize(&self, payload: &[u8]) -> Result<Personhendelse, AvroSerdeError> {
        self.0.deserialize(payload)
    }
}

#[derive(Clone, Debug)]
pub struct PersonhendelseSerializer(AvroDatumSerializer);

impl PersonhendelseSerializer {
    pub fn new() -> Result<Self, AvroSerdeError> {
        AvroDatumSerializer::new(PERSONHENDELSE_SCHEMA).map(Self)
    }

    pub fn serialize(&self, hendelse: &Personhendelse) -> Result<Vec<u8>, AvroSerdeError> {
        self.0.serialize(hendelse)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::leesah::opplysninger::adressebeskyttelse::{Adressebeskyttelse, Gradering};
    use crate::leesah::opplysninger::bostedsadresse::Bostedsadresse;
    use crate::leesah::opplysninger::common::adresse::{
        Koordinater, Matrikkeladresse, PostadresseIFrittFormat, Postboksadresse, UkjentBosted,
        UtenlandskAdresse, UtenlandskAdresseIFrittFormat, Vegadresse,
    };
    use crate::leesah::opplysninger::common::{KjoennType, Personnavn};
    use crate::leesah::opplysninger::delt_bosted::DeltBosted;
    use crate::leesah::opplysninger::doedfoedt_barn::DoedfoedtBarn;
    use crate::leesah::opplysninger::doedsfall::Doedsfall;
    use crate::leesah::opplysninger::falsk_identitet::FalskIdentitet;
    use crate::leesah::opplysninger::familierelasjon::Familierelasjon;
    use crate::leesah::opplysninger::foedsel::Foedsel;
    use crate::leesah::opplysninger::foedselsdato::Foedselsdato;
    use crate::leesah::opplysninger::folkeregisteridentifikator::Folkeregisteridentifikator;
    use crate::leesah::opplysninger::forelder_barn_relasjon::{
        ForelderBarnRelasjon, RelatertBiPerson,
    };
    use crate::leesah::opplysninger::innflytting::InnflyttingTilNorge;
    use crate::leesah::opplysninger::kontaktadresse::Kontaktadresse;
    use crate::leesah::opplysninger::navn::{Navn, OriginaltNavn};
    use crate::leesah::opplysninger::sikkerhetstiltak::{Kontaktperson, Sikkerhetstiltak};
    use crate::leesah::opplysninger::sivilstand::Sivilstand;
    use crate::leesah::opplysninger::statsborgerskap::Statsborgerskap;
    use crate::leesah::opplysninger::telefonnummer::Telefonnummer;
    use crate::leesah::opplysninger::utflytting::UtflyttingFraNorge;
    use crate::leesah::opplysninger::verge::{
        VergeEllerFullmektig, VergemaalEllerFremtidsfullmakt,
    };
    use chrono::{DateTime, NaiveDate};

    pub(crate) const IDENT: &str = "01017012345";

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn dato(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn vegadresse() -> Vegadresse {
        Vegadresse {
            matrikkel_id: s("1"),
            husnummer: s("2"),
            husbokstav: s("B"),
            bruksenhetsnummer: s("H0101"),
            adressenavn: s("Gate"),
            kommunenummer: s("0301"),
            bydelsnummer: s("030101"),
            tilleggsnavn: s("Tillegg"),
            postnummer: s("0150"),
            koordinater: Some(Koordinater {
                x: Some(1.5),
                y: Some(2.5),
                z: None,
            }),
        }
    }

    fn matrikkeladresse() -> Matrikkeladresse {
        Matrikkeladresse {
            matrikkel_id: s("1"),
            bruksenhetsnummer: s("H0101"),
            tilleggsnavn: s("Gård"),
            postnummer: s("0150"),
            kommunenummer: s("0301"),
            koordinater: Some(Koordinater {
                x: Some(1.0),
                y: None,
                z: Some(3.0),
            }),
        }
    }

    fn utenlandsk_adresse() -> UtenlandskAdresse {
        UtenlandskAdresse {
            adressenavn_nummer: s("Street 1"),
            bygning_etasje_leilighet: s("Apt 2"),
            postboks_nummer_navn: s("PO 3"),
            postkode: s("12345"),
            by_sted: s("By"),
            region_distrikt_omraade: s("Region"),
            landkode: "SWE".to_string(),
        }
    }

    fn personnavn() -> Personnavn {
        Personnavn {
            fornavn: "Ola".to_string(),
            mellomnavn: s("Mellom"),
            etternavn: "Nordmann".to_string(),
        }
    }

    pub(crate) fn fullt_utfylt() -> Personhendelse {
        Personhendelse {
            hendelse_id: "hendelse-1".to_string(),
            personidenter: vec![IDENT.to_string(), "1234567890123".to_string()],
            master: "FREG".to_string(),
            opprettet: DateTime::from_timestamp_millis(1_761_635_000_123).unwrap(),
            opplysningstype: Opplysningstype::BostedsadresseV1,
            endringstype: Endringstype::Korrigert,
            tidligere_hendelse_id: s("hendelse-0"),
            adressebeskyttelse: Some(Adressebeskyttelse {
                gradering: Gradering::StrengtFortroligUtland,
            }),
            delt_bosted: Some(DeltBosted {
                startdato_for_kontrakt: dato(2020, 1, 1),
                sluttdato_for_kontrakt: Some(dato(2021, 1, 1)),
                co_adressenavn: s("c/o"),
                vegadresse: Some(vegadresse()),
                matrikkeladresse: Some(matrikkeladresse()),
                ukjent_bosted: Some(UkjentBosted {
                    bostedskommune: s("0301"),
                }),
            }),
            doedfoedt_barn: Some(DoedfoedtBarn {
                dato: Some(dato(2019, 5, 5)),
            }),
            doedsfall: Some(Doedsfall {
                doedsdato: Some(dato(2026, 1, 1)),
            }),
            falsk_identitet: Some(FalskIdentitet { er_falsk: true }),
            foedsel: Some(Foedsel {
                foedselsaar: Some(1950),
                foedselsdato: Some(dato(1950, 1, 1)),
                foedeland: s("NOR"),
                foedested: s("Oslo"),
                foedekommune: s("0301"),
            }),
            foedselsdato: Some(Foedselsdato {
                foedselsaar: Some(1950),
                foedselsdato: Some(dato(1950, 1, 1)),
            }),
            forelder_barn_relasjon: Some(ForelderBarnRelasjon {
                relatert_persons_ident: s(IDENT),
                relatert_persons_rolle: "BARN".to_string(),
                min_rolle_for_person: s("MOR"),
                relatert_person_uten_folkeregisteridentifikator: Some(RelatertBiPerson {
                    navn: personnavn(),
                    foedselsdato: Some(dato(2010, 2, 3)),
                    statsborgerskap: s("SWE"),
                    kjoenn: Some(KjoennType::Kvinne),
                }),
            }),
            familierelasjon: Some(Familierelasjon {
                relatert_persons_ident: IDENT.to_string(),
                relatert_persons_rolle: "BARN".to_string(),
                min_rolle_for_person: s("FAR"),
            }),
            sivilstand: Some(Sivilstand {
                sivilstand_type: "GIFT".to_string(),
                gyldig_fra_og_med: Some(dato(2000, 6, 1)),
                relatert_ved_sivilstand: s(IDENT),
                bekreftelsesdato: Some(dato(2000, 6, 2)),
            }),
            vergemaal_eller_fremtidsfullmakt: Some(VergemaalEllerFremtidsfullmakt {
                vergemaal_type: s("voksen"),
                embete: s("Statsforvalteren"),
                verge_eller_fullmektig: VergeEllerFullmektig {
                    navn: Some(personnavn()),
                    motparts_personident: s(IDENT),
                    omfang: s("personligeOgOekonomiskeInteresser"),
                    omfanget_er_innen_personlig_omraade: Some(true),
                },
            }),
            utflytting_fra_norge: Some(UtflyttingFraNorge {
                tilflyttingsland: s("SWE"),
                tilflyttingssted_i_utlandet: s("Stockholm"),
                utflyttingsdato: Some(dato(2022, 3, 3)),
            }),
            innflytting_til_norge: Some(InnflyttingTilNorge {
                fraflyttingsland: s("DNK"),
                fraflyttingssted_i_utlandet: s("København"),
            }),
            folkeregisteridentifikator: Some(Folkeregisteridentifikator {
                identifikasjonsnummer: IDENT.to_string(),
                identifikator_type: "FNR".to_string(),
                status: "I_BRUK".to_string(),
            }),
            navn: Some(Navn {
                fornavn: "Ola".to_string(),
                mellomnavn: None,
                etternavn: "Nordmann".to_string(),
                forkortet_navn: s("Nordmann Ola"),
                originalt_navn: Some(OriginaltNavn {
                    fornavn: s("Olav"),
                    mellomnavn: None,
                    etternavn: s("Normann"),
                }),
                gyldig_fra_og_med: Some(dato(1960, 1, 1)),
            }),
            sikkerhetstiltak: Some(Sikkerhetstiltak {
                kontaktperson: Some(Kontaktperson {
                    personident: s("Z999999"),
                    norg_enhet: s("0315"),
                }),
                tiltakstype: "FYUS".to_string(),
                beskrivelse: "Fysisk utestengelse".to_string(),
                gyldig_fom: dato(2025, 1, 1),
                gyldig_tom: dato(2026, 12, 31),
            }),
            statsborgerskap: Some(Statsborgerskap {
                land: "NOR".to_string(),
                gyldig_fom: Some(dato(1950, 1, 1)),
                gyldig_tom: None,
                bekreftelsesdato: Some(dato(1950, 1, 2)),
            }),
            telefonnummer: Some(Telefonnummer {
                landskode: s("+47"),
                nummer: s("99999999"),
                prioritet: Some(1),
            }),
            kontaktadresse: Some(Kontaktadresse {
                gyldig_fra_og_med: Some(dato(2024, 1, 1)),
                gyldig_til_og_med: Some(dato(2025, 1, 1)),
                kontaktadresse_type: "Innland".to_string(),
                co_adressenavn: s("c/o"),
                postboksadresse: Some(Postboksadresse {
                    postbokseier: s("Eier"),
                    postboks: "123".to_string(),
                    postnummer: s("0150"),
                }),
                vegadresse: Some(vegadresse()),
                postadresse_i_fritt_format: Some(PostadresseIFrittFormat {
                    adresselinje1: s("Linje 1"),
                    adresselinje2: s("Linje 2"),
                    adresselinje3: s("Linje 3"),
                    postnummer: s("0150"),
                }),
                utenlandsk_adresse: Some(utenlandsk_adresse()),
                utenlandsk_adresse_i_fritt_format: Some(UtenlandskAdresseIFrittFormat {
                    adresselinje1: s("Line 1"),
                    adresselinje2: s("Line 2"),
                    adresselinje3: s("Line 3"),
                    postkode: s("12345"),
                    by_eller_stedsnavn: s("By"),
                    landkode: "SWE".to_string(),
                }),
            }),
            bostedsadresse: Some(Bostedsadresse {
                angitt_flyttedato: Some(dato(1969, 7, 20)),
                gyldig_fra_og_med: Some(dato(1969, 7, 21)),
                gyldig_til_og_med: None,
                co_adressenavn: s("c/o"),
                vegadresse: Some(vegadresse()),
                matrikkeladresse: Some(matrikkeladresse()),
                utenlandsk_adresse: Some(utenlandsk_adresse()),
                ukjent_bosted: Some(UkjentBosted {
                    bostedskommune: None,
                }),
            }),
        }
    }

    pub(crate) fn minimal() -> Personhendelse {
        Personhendelse {
            hendelse_id: "hendelse-2".to_string(),
            personidenter: vec![IDENT.to_string()],
            master: "PDL".to_string(),
            opprettet: DateTime::from_timestamp_millis(0).unwrap(),
            opplysningstype: Opplysningstype::DoedsfallV1,
            endringstype: Endringstype::Opprettet,
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
            bostedsadresse: None,
        }
    }

    #[test]
    fn round_trip() {
        let ser = PersonhendelseSerializer::new().unwrap();
        let de = PersonhendelseDeserializer::new().unwrap();
        for hendelse in [fullt_utfylt(), minimal()] {
            let bytes = ser.serialize(&hendelse).unwrap();
            assert_eq!(de.deserialize(&bytes).unwrap(), hendelse);
        }
    }

    #[test]
    fn debug_skjuler_personopplysninger() {
        let debug = format!("{:?}", fullt_utfylt());
        assert!(debug.contains("hendelse-1"));
        for pii in [IDENT, "Ola", "Nordmann", "Gate", "STRENGT", "99999999"] {
            assert!(!debug.contains(pii), "Debug inneholder {pii}: {debug}");
        }
    }
}
