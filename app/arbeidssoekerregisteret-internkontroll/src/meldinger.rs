use std::collections::HashMap;

use bekreftelse_interne_hendelser::BekreftelseHendelse;
use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
use eksterne_hendelser::bekreftelse::paa_vegne_av::{Handling, PaaVegneAv};
use eksterne_hendelser::egenvurdering::Egenvurdering;
use eksterne_hendelser::opplysninger::Opplysninger;
use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::profilering::Profilering;
use interne_hendelser::InterneHendelser;
use paw_kafka::topics::Topic;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::avro::AvroDecoder;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use schema_registry_converter::error::SRCError;
use schema_registry_converter::schema_registry_common::{BytesResult, get_bytes_result};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::database::payload::Meldingstype;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DekodetMelding {
    pub meldingstype: Meldingstype,
    pub periode_id: Option<Uuid>,
    pub arbeidssoeker_id: Option<i64>,
}

fn melding(
    meldingstype: Meldingstype,
    periode_id: Option<Uuid>,
    arbeidssoeker_id: i64,
) -> DekodetMelding {
    DekodetMelding {
        meldingstype,
        periode_id,
        arbeidssoeker_id: Some(arbeidssoeker_id),
    }
}

/// Returns `None` when the payload is not a valid Hendelselogg event.
pub fn dekod_hendelselogg(payload: Option<&[u8]>) -> Option<DekodetMelding> {
    use Meldingstype as T;
    Some(
        match serde_json::from_slice::<InterneHendelser>(payload?).ok()? {
            InterneHendelser::Startet(h) => melding(T::HendelseStartet, Some(h.hendelse_id), h.id),
            InterneHendelser::Avsluttet(h) => melding(T::HendelseAvsluttet, h.periode_id, h.id),
            InterneHendelser::Avvist(h) => melding(T::HendelseAvvist, None, h.id),
            InterneHendelser::AvvistStoppAvPeriode(h) => {
                melding(T::HendelseAvvistStoppAvPeriode, None, h.id)
            }
            InterneHendelser::ArbeidssoekerIdFlettetInn(h) => {
                melding(T::HendelseArbeidssoekerIdFlettetInn, None, h.id)
            }
            InterneHendelser::AutomatiskIdMergeIkkeMulig(h) => {
                melding(T::HendelseAutomatiskIdMergeIkkeMulig, None, h.id)
            }
            InterneHendelser::IdentitetsnummerSammenslaatt(h) => melding(
                T::HendelseIdentitetsnummerSammenslaatt,
                None,
                h.flyttet_til_arbeidssoeker_id,
            ),
            InterneHendelser::OpplysningerOmArbeidssoekerMottatt(h) => {
                melding(T::HendelseOpplysningerOmArbeidssoekerMottatt, None, h.id)
            }
        },
    )
}

/// Returns `None` when the payload is not a valid BekreftelseHendelseLogg event.
pub fn dekod_bekreftelse_hendelselogg(payload: Option<&[u8]>) -> Option<DekodetMelding> {
    use BekreftelseHendelse as H;
    use Meldingstype as T;
    let hendelse = BekreftelseHendelse::from_slice(payload?).ok()?;
    let meldingstype = match &hendelse {
        H::LeveringsfristUtloept(_) => T::BekreftelseHendelseLeveringsfristUtloept,
        H::EksternGracePeriodeUtloept(_) => T::BekreftelseHendelseEksternGracePeriodeUtloept,
        H::RegisterGracePeriodeUtloept(_) => T::BekreftelseHendelseRegisterGracePeriodeUtloept,
        H::BekreftelseTilgjengelig(_) => T::BekreftelseHendelseTilgjengelig,
        H::BekreftelseMeldingMottatt(_) => T::BekreftelseHendelseMeldingMottatt,
        H::PeriodeAvsluttet(_) => T::BekreftelseHendelsePeriodeAvsluttet,
        H::RegisterGracePeriodeGjenstaaendeTid(_) => {
            T::BekreftelseHendelseRegisterGracePeriodeGjenstaaendeTid
        }
        H::BaOmAaAvsluttePeriode(_) => T::BekreftelseHendelseBaOmAaAvsluttePeriode,
        H::BekreftelsePaaVegneAvStartet(_) => T::BekreftelseHendelsePaaVegneAvStartet,
        H::RegisterGracePeriodeUtloeptEtterEksternInnsamling(_) => {
            T::BekreftelseHendelseGraceUtloeptEtterEksternInnsamling
        }
    };
    Some(melding(
        meldingstype,
        Some(hendelse.periode_id()),
        hendelse.arbeidssoeker_id(),
    ))
}

#[derive(Debug, Clone, Copy)]
enum Dekoder {
    Hendelselogg,
    BekreftelseHendelselogg,
    Periode,
    Opplysninger,
    Profilering,
    Egenvurdering,
    Bekreftelse,
    PaaVegneAv,
}

fn dekoder_for(topic: &Topic) -> Dekoder {
    match topic {
        Topic::Hendelselogg => Dekoder::Hendelselogg,
        Topic::BekreftelseHendelseLogg => Dekoder::BekreftelseHendelselogg,
        Topic::Periode => Dekoder::Periode,
        Topic::Opplysninger => Dekoder::Opplysninger,
        Topic::Profilering => Dekoder::Profilering,
        Topic::Egenvurdering => Dekoder::Egenvurdering,
        Topic::Bekreftelse => Dekoder::Bekreftelse,
        Topic::PaaVegneAv => Dekoder::PaaVegneAv,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DekodingsFeil {
    #[error("no payload decoder for topic {0}")]
    UkjentTopic(String),
    #[error("schema registry error: {0}")]
    SchemaRegistry(SRCError),
}

/// Decodes the payload of every subscribed topic. `Ok(None)` means the payload is invalid.
pub struct Dekodere {
    per_topic: HashMap<String, Dekoder>,
    avro: AvroDecoder<'static>,
}

impl Dekodere {
    pub fn new<'a>(
        topics: impl IntoIterator<Item = (&'a str, &'a Topic)>,
        schema_registry: SrSettings,
    ) -> Self {
        Self {
            per_topic: topics
                .into_iter()
                .map(|(navn, topic)| (navn.to_string(), dekoder_for(topic)))
                .collect(),
            avro: AvroDecoder::new(schema_registry),
        }
    }

    pub async fn dekod(&self, msg: &OwnedMessage) -> Result<Option<DekodetMelding>, DekodingsFeil> {
        let dekoder = self
            .per_topic
            .get(msg.topic())
            .ok_or_else(|| DekodingsFeil::UkjentTopic(msg.topic().to_string()))?;
        let payload = msg.payload();
        Ok(match dekoder {
            Dekoder::Hendelselogg => dekod_hendelselogg(payload),
            Dekoder::BekreftelseHendelselogg => dekod_bekreftelse_hendelselogg(payload),
            Dekoder::Periode => self.avro::<Periode>(payload).await?.map(|p| {
                let meldingstype = match p.avsluttet {
                    None => Meldingstype::PeriodeStartet,
                    Some(_) => Meldingstype::PeriodeAvsluttet,
                };
                uten_arbeidssoeker(meldingstype, p.id)
            }),
            Dekoder::Opplysninger => self
                .avro::<Opplysninger>(payload)
                .await?
                .map(|o| uten_arbeidssoeker(Meldingstype::Opplysninger, o.periode_id)),
            Dekoder::Profilering => self
                .avro::<Profilering>(payload)
                .await?
                .map(|p| uten_arbeidssoeker(Meldingstype::Profilering, p.periode_id)),
            Dekoder::Egenvurdering => self
                .avro::<Egenvurdering>(payload)
                .await?
                .map(|e| uten_arbeidssoeker(Meldingstype::Egenvurdering, e.periode_id)),
            Dekoder::Bekreftelse => self
                .avro::<Bekreftelse>(payload)
                .await?
                .map(|b| uten_arbeidssoeker(Meldingstype::Bekreftelse, b.periode_id)),
            Dekoder::PaaVegneAv => self.avro::<PaaVegneAv>(payload).await?.map(|p| {
                let meldingstype = match p.handling {
                    Handling::Start(_) => Meldingstype::PaaVegneAvStart,
                    Handling::Stopp(_) => Meldingstype::PaaVegneAvStopp,
                };
                uten_arbeidssoeker(meldingstype, p.periode_id)
            }),
        })
    }

    /// Only a payload that cannot be read is invalid. Any other schema registry error,
    /// such as a lookup or authentication failure, is returned so the record is retried.
    async fn avro<T: DeserializeOwned>(
        &self,
        payload: Option<&[u8]>,
    ) -> Result<Option<T>, DekodingsFeil> {
        if !matches!(get_bytes_result(payload), BytesResult::Valid(..)) {
            return Ok(None);
        }
        match self.avro.decode(payload).await {
            Ok(dekodet) => Ok(apache_avro::from_value(&dekodet.value).ok()),
            Err(feil) if feil.error == UGYLDIGE_AVRO_BYTES => Ok(None),
            Err(feil) => Err(DekodingsFeil::SchemaRegistry(feil)),
        }
    }
}

const UGYLDIGE_AVRO_BYTES: &str = "Could not transform bytes using schema";

fn uten_arbeidssoeker(meldingstype: Meldingstype, periode_id: Uuid) -> DekodetMelding {
    DekodetMelding {
        meldingstype,
        periode_id: Some(periode_id),
        arbeidssoeker_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = r#""metadata": {"tidspunkt": 1770897349.3, "utfoertAv": {"type": "SYSTEM", "id": "x"}, "kilde": "x", "aarsak": "x"}"#;

    fn json(felter: &str) -> String {
        format!(
            r#"{{"hendelseId": "00000000-0000-0000-0000-000000000001", "identitetsnummer": "1", {METADATA}, {felter}}}"#
        )
    }

    fn dekod(felter: &str) -> Option<DekodetMelding> {
        dekod_hendelselogg(Some(json(felter).as_bytes()))
    }

    #[test]
    fn startet_har_hendelse_id_som_periode_id() {
        assert_eq!(
            dekod(r#""hendelseType": "intern.v1.startet", "id": 42"#),
            Some(melding(
                Meldingstype::HendelseStartet,
                Some(Uuid::from_u128(1)),
                42
            ))
        );
    }

    #[test]
    fn avsluttet_har_periode_id_bare_naar_den_er_oppgitt() {
        assert_eq!(
            dekod(r#""hendelseType": "intern.v1.avsluttet", "id": 42"#),
            Some(melding(Meldingstype::HendelseAvsluttet, None, 42))
        );
        assert_eq!(
            dekod(
                r#""hendelseType": "intern.v1.avsluttet", "id": 42, "periodeId": "00000000-0000-0000-0000-000000000002""#
            ),
            Some(melding(
                Meldingstype::HendelseAvsluttet,
                Some(Uuid::from_u128(2)),
                42
            ))
        );
    }

    #[test]
    fn sammenslaaing_tilhoerer_arbeidssoekeren_som_fortsetter() {
        assert_eq!(
            dekod(
                r#""hendelseType": "intern.v1.arbeidssoeker_id_flettet_inn", "id": 42, "kilde": {"arbeidssoekerId": 7, "identitetsnummer": ["2"]}"#
            ),
            Some(melding(
                Meldingstype::HendelseArbeidssoekerIdFlettetInn,
                None,
                42
            ))
        );
        assert_eq!(
            dekod(
                r#""hendelseType": "intern.v1.identitetsnummer_sammenslaatt", "id": 7, "flyttedeIdentitetsnumre": ["2"], "flyttetTilArbeidssoekerId": 42"#
            ),
            Some(melding(
                Meldingstype::HendelseIdentitetsnummerSammenslaatt,
                None,
                42
            ))
        );
    }

    #[test]
    fn ugyldig_hendelselogg_gir_none() {
        assert_eq!(dekod_hendelselogg(Some(b"ikke json")), None);
        assert_eq!(dekod_hendelselogg(None), None);
        assert_eq!(
            dekod(r#""hendelseType": "intern.v1.ny_type", "id": 42"#),
            None
        );
        assert_eq!(dekod(r#""hendelseType": "intern.v1.startet""#), None);
    }

    #[test]
    fn bekreftelse_hendelse_har_periode_og_arbeidssoeker() {
        let json = br#"{"hendelseType": "bekreftelse.melding_mottatt", "hendelseId": "00000000-0000-0000-0000-000000000009", "periodeId": "00000000-0000-0000-0000-000000000001", "arbeidssoekerId": 42, "hendelseTidspunkt": 1770897349.3, "bekreftelseId": "00000000-0000-0000-0000-000000000008"}"#;
        assert_eq!(
            dekod_bekreftelse_hendelselogg(Some(json)),
            Some(melding(
                Meldingstype::BekreftelseHendelseMeldingMottatt,
                Some(Uuid::from_u128(1)),
                42
            ))
        );
        assert_eq!(dekod_bekreftelse_hendelselogg(Some(b"ikke json")), None);
    }
}

#[cfg(test)]
mod avro_tests {
    use super::*;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use rdkafka::Timestamp;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::eksterne_hendelser::*;

    const PERIODE: &str = "paw.arbeidssokerperioder-v1";
    const OPPLYSNINGER: &str = "paw.opplysninger-om-arbeidssoeker-v1";
    const PROFILERING: &str = "paw.arbeidssoker-profilering-v1";
    const EGENVURDERING: &str = "paw.arbeidssoeker-egenvurdering-v1";
    const BEKREFTELSE: &str = "paw.arbeidssoker-bekreftelse-v1";
    const PAA_VEGNE_AV: &str = "paw.arbeidssoker-bekreftelse-paavegneav-v1";

    fn dekodere(settings: SrSettings) -> Dekodere {
        Dekodere::new(
            [
                (PERIODE, &Topic::Periode),
                (OPPLYSNINGER, &Topic::Opplysninger),
                (PROFILERING, &Topic::Profilering),
                (EGENVURDERING, &Topic::Egenvurdering),
                (BEKREFTELSE, &Topic::Bekreftelse),
                (PAA_VEGNE_AV, &Topic::PaaVegneAv),
            ],
            settings,
        )
    }

    fn melding(topic: &str, payload: Vec<u8>) -> OwnedMessage {
        OwnedMessage::new(
            Some(payload),
            None,
            topic.into(),
            Timestamp::NotAvailable,
            0,
            0,
            None,
        )
    }

    fn forventet(meldingstype: Meldingstype, periode_id: Uuid) -> Option<DekodetMelding> {
        Some(DekodetMelding {
            meldingstype,
            periode_id: Some(periode_id),
            arbeidssoeker_id: None,
        })
    }

    #[tokio::test]
    async fn alle_avro_topics_gir_meldingstype_og_periode_id() {
        let mut server = mockito::Server::new_async().await;
        let mock = create_schema_registry_mock(&mut server).await.unwrap();
        let generator = AvroGenerator::new(mock.schema_registry_settings.clone());
        let dekodere = dekodere(mock.schema_registry_settings.clone());
        let periode_id = Uuid::from_u128(1);
        let id = Uuid::from_u128(2);
        let fnr = "12345678901";

        let tilfeller = [
            (
                generator
                    .create_avro_payload(PERIODE, create_dummy_start_periode(fnr, periode_id, None))
                    .await,
                PERIODE,
                Meldingstype::PeriodeStartet,
            ),
            (
                generator
                    .create_avro_payload(
                        PERIODE,
                        create_dummy_avslutt_periode(fnr, periode_id, None, None),
                    )
                    .await,
                PERIODE,
                Meldingstype::PeriodeAvsluttet,
            ),
            (
                generator
                    .create_avro_payload(
                        OPPLYSNINGER,
                        create_dummy_opplysninger(fnr, periode_id, id),
                    )
                    .await,
                OPPLYSNINGER,
                Meldingstype::Opplysninger,
            ),
            (
                generator
                    .create_avro_payload(
                        PROFILERING,
                        create_dummy_profilering(fnr, periode_id, id, id),
                    )
                    .await,
                PROFILERING,
                Meldingstype::Profilering,
            ),
            (
                generator
                    .create_avro_payload(
                        EGENVURDERING,
                        create_dummy_egenvurdering(fnr, periode_id, id, id),
                    )
                    .await,
                EGENVURDERING,
                Meldingstype::Egenvurdering,
            ),
            (
                generator
                    .create_avro_payload(
                        BEKREFTELSE,
                        create_dummy_bekreftelse(fnr, periode_id, id, None, None, true, true),
                    )
                    .await,
                BEKREFTELSE,
                Meldingstype::Bekreftelse,
            ),
            (
                generator
                    .create_avro_payload(
                        PAA_VEGNE_AV,
                        create_dummy_start_paavegneav(periode_id, Bekreftelsesloesning::Dagpenger),
                    )
                    .await,
                PAA_VEGNE_AV,
                Meldingstype::PaaVegneAvStart,
            ),
            (
                generator
                    .create_avro_payload(
                        PAA_VEGNE_AV,
                        create_dummy_stopp_paavegneav(periode_id, Bekreftelsesloesning::Dagpenger),
                    )
                    .await,
                PAA_VEGNE_AV,
                Meldingstype::PaaVegneAvStopp,
            ),
        ];
        for (payload, topic, meldingstype) in tilfeller {
            assert_eq!(
                dekodere.dekod(&melding(topic, payload)).await.unwrap(),
                forventet(meldingstype, periode_id),
                "{meldingstype:?}"
            );
        }
    }

    #[tokio::test]
    async fn ugyldig_avro_payload_er_ugyldig() {
        let mut server = mockito::Server::new_async().await;
        let mock = create_schema_registry_mock(&mut server).await.unwrap();
        let generator = AvroGenerator::new(mock.schema_registry_settings.clone());
        let dekodere = dekodere(mock.schema_registry_settings.clone());

        let mut avkuttet = generator
            .create_avro_payload(
                PERIODE,
                create_dummy_start_periode("12345678901", Uuid::from_u128(1), None),
            )
            .await;
        avkuttet.truncate(8);
        for payload in [b"{}".to_vec(), vec![0, 0, 0], avkuttet] {
            assert_eq!(
                dekodere.dekod(&melding(PERIODE, payload)).await.unwrap(),
                None
            );
        }
        let tom = OwnedMessage::new(
            None,
            None,
            PERIODE.into(),
            Timestamp::NotAvailable,
            0,
            0,
            None,
        );
        assert_eq!(dekodere.dekod(&tom).await.unwrap(), None);
    }

    #[tokio::test]
    async fn feil_fra_schema_registry_er_ikke_ugyldig_payload() {
        let mut server = mockito::Server::new_async().await;
        let _avvist = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(401)
            .create_async()
            .await;
        let settings = SrSettings::new_builder(server.url())
            .no_proxy()
            .build()
            .unwrap();
        let avvist = dekodere(settings);
        let payload = vec![0, 0, 0, 0, 1, 2];
        assert!(matches!(
            avvist.dekod(&melding(PERIODE, payload)).await,
            Err(DekodingsFeil::SchemaRegistry(_))
        ));

        let utilgjengelig = dekodere(
            SrSettings::new_builder("http://127.0.0.1:1".into())
                .no_proxy()
                .build()
                .unwrap(),
        );
        assert!(matches!(
            utilgjengelig
                .dekod(&melding(PERIODE, vec![0, 0, 0, 0, 1, 2]))
                .await,
            Err(DekodingsFeil::SchemaRegistry(_))
        ));
    }

    #[tokio::test]
    async fn topic_uten_dekoder_er_feil() {
        let dekodere = dekodere(SrSettings::new("http://127.0.0.1:1".into()));
        assert!(matches!(
            dekodere.dekod(&melding("ukjent", vec![])).await,
            Err(DekodingsFeil::UkjentTopic(_))
        ));
    }
}
