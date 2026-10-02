//! Rust-motstykke til Kotlin-modulen `domain/bekreftelse-interne-hendelser`.
//! Topicet inneholder Jackson-serialisert JSON, med `hendelseType` som diskriminator.
//! Ukjent `hendelseType` gir feil, slik som i Kotlin-deserialisereren.

pub mod duration;
pub mod hendelser;
pub mod instant;

pub use hendelser::*;
pub use interne_hendelser::vo::{Bruker, BrukerType};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "hendelseType")]
pub enum BekreftelseHendelse {
    #[serde(rename = "bekreftelse.leveringsfrist_utloept")]
    LeveringsfristUtloept(LeveringsfristUtloept),
    #[serde(rename = "bekreftelse.ekstern_grace_periode_utloept")]
    EksternGracePeriodeUtloept(EksternGracePeriodeUtloept),
    #[serde(rename = "bekreftelse.register_grace_periode_utloept")]
    RegisterGracePeriodeUtloept(RegisterGracePeriodeUtloept),
    #[serde(rename = "bekreftelse.tilgjengelig")]
    BekreftelseTilgjengelig(BekreftelseTilgjengelig),
    #[serde(rename = "bekreftelse.melding_mottatt")]
    BekreftelseMeldingMottatt(BekreftelseMeldingMottatt),
    #[serde(rename = "bekreftelse.periode_avsluttet")]
    PeriodeAvsluttet(PeriodeAvsluttet),
    #[serde(rename = "bekreftelse.register_grace_periode_gjenstaande_tid")]
    RegisterGracePeriodeGjenstaaendeTid(RegisterGracePeriodeGjenstaaendeTid),
    #[serde(rename = "bekreftelse.ba_om_aa_avslutte_periode")]
    BaOmAaAvsluttePeriode(BaOmAaAvsluttePeriode),
    #[serde(rename = "bekreftelse.paa_vegne_av_startet")]
    BekreftelsePaaVegneAvStartet(BekreftelsePaaVegneAvStartet),
    #[serde(rename = "bekreftelse.register_grace_periode_utloept_etter_ekstern_innsamling")]
    RegisterGracePeriodeUtloeptEtterEksternInnsamling(
        RegisterGracePeriodeUtloeptEtterEksternInnsamling,
    ),
}

impl BekreftelseHendelse {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn periode_id(&self) -> uuid::Uuid {
        match self {
            Self::LeveringsfristUtloept(h) => h.periode_id,
            Self::EksternGracePeriodeUtloept(h) => h.periode_id,
            Self::RegisterGracePeriodeUtloept(h) => h.periode_id,
            Self::BekreftelseTilgjengelig(h) => h.periode_id,
            Self::BekreftelseMeldingMottatt(h) => h.periode_id,
            Self::PeriodeAvsluttet(h) => h.periode_id,
            Self::RegisterGracePeriodeGjenstaaendeTid(h) => h.periode_id,
            Self::BaOmAaAvsluttePeriode(h) => h.periode_id,
            Self::BekreftelsePaaVegneAvStartet(h) => h.periode_id,
            Self::RegisterGracePeriodeUtloeptEtterEksternInnsamling(h) => h.periode_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    const IDS: &str = r#""hendelseId": "123e4567-e89b-12d3-a456-426614174000",
        "periodeId": "223e4567-e89b-12d3-a456-426614174000",
        "arbeidssoekerId": 1234567890,
        "hendelseTidspunkt": 1770897349.305000000"#;

    fn parse(json: &str) -> BekreftelseHendelse {
        serde_json::from_str(json).expect("kunne ikke deserialisere")
    }

    #[test]
    fn leveringsfrist_utloept() {
        let hendelse = parse(&format!(
            r#"{{"hendelseType": "bekreftelse.leveringsfrist_utloept", {IDS},
            "bekreftelseId": "323e4567-e89b-12d3-a456-426614174000",
            "leveringsfrist": 1770897349.5}}"#
        ));
        let BekreftelseHendelse::LeveringsfristUtloept(h) = hendelse else {
            panic!("feil variant");
        };
        assert_eq!(h.arbeidssoeker_id, 1234567890);
        assert_eq!(h.hendelse_tidspunkt.timestamp(), 1770897349);
        assert_eq!(h.leveringsfrist.timestamp_millis(), 1770897349500);
    }

    #[test]
    fn ba_om_aa_avslutte_periode_med_bruker_og_default_kilde() {
        let hendelse = parse(&format!(
            r#"{{"hendelseType": "bekreftelse.ba_om_aa_avslutte_periode", {IDS},
            "utfoertAv": {{"type": "SLUTTBRUKER", "id": "x", "sikkerhetsnivaa": null}}}}"#
        ));
        let BekreftelseHendelse::BaOmAaAvsluttePeriode(h) = hendelse else {
            panic!("feil variant");
        };
        assert_eq!(h.utfoert_av.bruker_type, BrukerType::Sluttbruker);
        assert_eq!(h.utfoert_av.sikkerhetsnivaa, None);
        assert_eq!(h.kilde, "");
    }

    #[test]
    fn gjenstaande_tid_leses_som_sekunder() {
        let hendelse = parse(&format!(
            r#"{{"hendelseType": "bekreftelse.register_grace_periode_gjenstaande_tid", {IDS},
            "bekreftelseId": "323e4567-e89b-12d3-a456-426614174000",
            "gjenstaandeTid": 86400.000000000}}"#
        ));
        let BekreftelseHendelse::RegisterGracePeriodeGjenstaaendeTid(h) = hendelse else {
            panic!("feil variant");
        };
        assert_eq!(h.gjenstaande_tid, TimeDelta::days(1));
    }

    #[test]
    fn instant_som_iso_tekst_og_heltall() {
        let tekst = parse(
            r#"{"hendelseType": "bekreftelse.periode_avsluttet",
            "hendelseId": "123e4567-e89b-12d3-a456-426614174000",
            "periodeId": "223e4567-e89b-12d3-a456-426614174000",
            "arbeidssoekerId": 1, "hendelseTidspunkt": "2026-02-12T12:00:00Z"}"#,
        );
        let heltall = parse(
            r#"{"hendelseType": "bekreftelse.periode_avsluttet",
            "hendelseId": "123e4567-e89b-12d3-a456-426614174000",
            "periodeId": "223e4567-e89b-12d3-a456-426614174000",
            "arbeidssoekerId": 1, "hendelseTidspunkt": 1770897600}"#,
        );
        assert_eq!(tekst, heltall);
    }

    #[test]
    fn roundtrip_alle_varianter_med_ekstra_felt_fra_kotlin() {
        // Kotlin skriver også `hendelseType` som egenskap på selve klassen.
        let json = format!(
            r#"{{"hendelseType": "bekreftelse.tilgjengelig", {IDS},
            "bekreftelseId": "323e4567-e89b-12d3-a456-426614174000",
            "gjelderFra": 1770897349.5, "gjelderTil": 1771897349.25}}"#
        );
        let hendelse = parse(&json);
        let tilbake: BekreftelseHendelse =
            serde_json::from_str(&serde_json::to_string(&hendelse).unwrap()).unwrap();
        assert_eq!(hendelse, tilbake);
    }

    #[test]
    fn ukjent_hendelse_type_gir_feil() {
        assert!(
            serde_json::from_str::<BekreftelseHendelse>(
                r#"{"hendelseType": "bekreftelse.ukjent"}"#
            )
            .is_err()
        );
    }
}
