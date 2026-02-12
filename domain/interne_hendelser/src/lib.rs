pub mod aarsak;
pub mod arbeidssoeker_id_flettet_inn;
pub mod automatisk_id_merge_ikke_mulig;
pub mod avsluttet;
pub mod avvist;
pub mod avvist_stopp_av_periode;
pub mod identitetsnummer_sammenslaatt;
pub mod opplysninger_om_arbeidssoeker_mottatt;
pub mod startet;
pub mod vo;

pub use aarsak::Aarsak;
pub use arbeidssoeker_id_flettet_inn::{
    ARBEIDSSOEKER_ID_FLETTET_INN, ArbeidssoekerIdFlettetInn, Kilde,
};
pub use automatisk_id_merge_ikke_mulig::{
    AUTOMATISK_ID_MERGE_IKKE_MULIG, Alias, AutomatiskIdMergeIkkeMulig, PeriodeRad,
};
pub use avsluttet::{AVSLUTTET_HENDELSE_TYPE, Avsluttet};
pub use avvist::{AVVIST_HENDELSE_TYPE, Avvist};
pub use avvist_stopp_av_periode::{AVVIST_STOPP_AV_PERIODE_HENDELSE_TYPE, AvvistStoppAvPeriode};
pub use identitetsnummer_sammenslaatt::{
    IDENTITETSNUMMER_SAMMENSLAATT_HENDELSE_TYPE, IdentitetsnummerSammenslaatt,
};
pub use opplysninger_om_arbeidssoeker_mottatt::{
    OPPLYSNINGER_OM_ARBEIDSSOEKER_HENDELSE_TYPE, OpplysningerOmArbeidssoekerMottatt,
};
pub use startet::{STARTET_HENDELSE_TYPE, Startet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "hendelseType")]
pub enum InterneHendelser {
    #[serde(rename = "intern.v1.startet")]
    Startet(Startet),
    #[serde(rename = "intern.v1.avsluttet")]
    Avsluttet(Avsluttet),
    #[serde(rename = "intern.v1.avvist")]
    Avvist(Avvist),
    #[serde(rename = "intent.v1.avvist_stopp_av_periode")]
    AvvistStoppAvPeriode(AvvistStoppAvPeriode),
    #[serde(rename = "intern.v1.arbeidssoeker_id_flettet_inn")]
    ArbeidssoekerIdFlettetInn(ArbeidssoekerIdFlettetInn),
    #[serde(rename = "intern.v1.automatisk_id_merge_ikke_mulig")]
    AutomatiskIdMergeIkkeMulig(AutomatiskIdMergeIkkeMulig),
    #[serde(rename = "intern.v1.identitetsnummer_sammenslaatt")]
    IdentitetsnummerSammenslaatt(IdentitetsnummerSammenslaatt),
    #[serde(rename = "intern.v1.opplysninger_om_arbeidssoeker")]
    OpplysningerOmArbeidssoekerMottatt(OpplysningerOmArbeidssoekerMottatt),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_startet_event() {
        let json = r#"{
            "hendelseType": "intern.v1.startet",
            "hendelseId": "123e4567-e89b-12d3-a456-426614174000",
            "id": 42,
            "identitetsnummer": "12345678901",
            "metadata": {
                "tidspunkt": "2026-02-12T10:30:00Z",
                "utfoertAv": {
                    "type": "SYSTEM",
                    "id": "test-system"
                },
                "kilde": "test-kilde",
                "aarsak": "test-aarsak"
            },
            "opplysninger": []
        }"#;

        let hendelse: InterneHendelser =
            serde_json::from_str(json).expect("Failed to deserialize startet event");

        match hendelse {
            InterneHendelser::Startet(startet) => {
                assert_eq!(startet.id, 42);
                assert_eq!(startet.identitetsnummer, "12345678901");
                assert_eq!(startet.metadata.kilde, "test-kilde");
                assert_eq!(startet.metadata.aarsak, "test-aarsak");
                assert!(startet.opplysninger.is_empty());
            }
            _ => panic!("Expected Startet variant"),
        }
    }
}
