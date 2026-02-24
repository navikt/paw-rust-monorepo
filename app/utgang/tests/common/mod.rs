use std::collections::hash_set;

use chrono::DateTime;
use interne_hendelser::{
    Startet,
    vo::{Bruker as HendelseBruker, Metadata as HendelseMetadata, Opplysning},
};
use utgang::kafka::periode_deserializer::{Bruker, BrukerType, Metadata, Periode};
use uuid::Uuid;

pub fn main_avro_periode() -> Periode {
    Periode {
        id: Uuid::new_v4(),
        identitetsnummer: "12345678901".to_string(),
        startet: main_avro_metadata(),
        avsluttet: Option::None,
    }
}

pub fn main_avro_metadata() -> Metadata {
    Metadata {
        tidspunkt: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .to_utc(),
        utfoert_av: main_avro_bruker(),
        kilde: "test-kilde".to_string(),
        aarsak: "test-aarsak".to_string(),
        tidspunkt_fra_kilde: Option::None,
    }
}

pub fn main_avro_bruker() -> Bruker {
    Bruker {
        id: "12345678910".to_string(),
        bruker_type: main_avro_bruker_type(),
        sikkerhetsnivaa: Option::None,
    }
}

pub fn main_avro_bruker_type() -> BrukerType {
    BrukerType::Sluttbruker
}

pub fn hendelse_startet() -> Startet {
    Startet {
        hendelse_id: Uuid::new_v4(),
        id: 1,
        identitetsnummer: "12345678901".to_string(),
        metadata: hendelse_metadata(),
        opplysninger: hash_set::HashSet::from([
            Opplysning::ErOver18Aar,
            Opplysning::HarNorskAdresse,
            Opplysning::BosattEtterFregLoven,
            Opplysning::SammeSomInnloggetBruker,
            Opplysning::SisteFlyttingVarInnTilNorge,
        ]),
    }
}

pub fn hendelse_metadata() -> HendelseMetadata {
    HendelseMetadata {
        tidspunkt: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .to_utc(),
        utfoert_av: hendelse_bruker(),
        kilde: "test-kilde".to_string(),
        aarsak: "test-aarsak".to_string(),
        tidspunkt_fra_kilde: Option::None,
    }
}

pub fn hendelse_bruker() -> HendelseBruker {
    HendelseBruker {
        id: "12345678910".to_string(),
        bruker_type: interne_hendelser::vo::BrukerType::Sluttbruker,
        sikkerhetsnivaa: Option::None,
    }
}
