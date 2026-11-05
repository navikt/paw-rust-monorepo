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

use crate::features::innlesing::metadata::arbeidssoeker_id::ArbeidssoekerId;
use crate::features::innlesing::metadata::kilde::{Opphav, opphav_fra_header, opphav_fra_hendelse};
use crate::features::innlesing::metadata::payload::Meldingstype;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DekodetMelding {
    pub meldingstype: Meldingstype,
    pub periode_id: Option<Uuid>,
    pub arbeidssoeker_id: Option<ArbeidssoekerId>,
    pub opphav: Opphav,
}

fn melding(
    meldingstype: Meldingstype,
    periode_id: Option<Uuid>,
    arbeidssoeker_id: i64,
    opphav: Opphav,
) -> DekodetMelding {
    DekodetMelding {
        meldingstype,
        periode_id,
        arbeidssoeker_id: Some(ArbeidssoekerId::new(arbeidssoeker_id)),
        opphav,
    }
}

/// Returns `None` when the payload is not a valid Hendelselogg event.
pub fn dekod_hendelselogg(payload: Option<&[u8]>) -> Option<DekodetMelding> {
    use Meldingstype as T;
    let hendelse = serde_json::from_slice::<InterneHendelser>(payload?).ok()?;
    // Only Startet and Avsluttet can start a chain.
    let opphav = match &hendelse {
        InterneHendelser::Startet(h) => opphav_fra_hendelse(&h.metadata.kilde),
        InterneHendelser::Avsluttet(h) => opphav_fra_hendelse(&h.metadata.kilde),
        _ => Opphav::IKKE_AKTUELT,
    };
    let melding = |meldingstype, periode_id, arbeidssoeker_id| {
        melding(meldingstype, periode_id, arbeidssoeker_id, opphav)
    };
    Some(match hendelse {
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
    })
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
    // Timers in bekreftelse-tjeneste start a chain; the rest follow from another message.
    let opphav = match &hendelse {
        H::LeveringsfristUtloept(_)
        | H::EksternGracePeriodeUtloept(_)
        | H::RegisterGracePeriodeUtloept(_)
        | H::RegisterGracePeriodeGjenstaaendeTid(_)
        | H::BekreftelseTilgjengelig(_) => Opphav::BEKREFTELSE_TJENESTE,
        _ => Opphav::IKKE_AKTUELT,
    };
    Some(melding(
        meldingstype,
        Some(hendelse.periode_id()),
        hendelse.arbeidssoeker_id(),
        opphav,
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
    pub fn new<'a>(topics: Vec<(&String, &Topic)>, schema_registry: SrSettings) -> Self {
        Self {
            per_topic: topics
                .into_iter()
                .map(|(navn, topic)| (navn.clone(), dekoder_for(topic)))
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
                uten_arbeidssoeker(meldingstype, p.id, Opphav::IKKE_AKTUELT)
            }),
            Dekoder::Opplysninger => self.avro::<Opplysninger>(payload).await?.map(|o| {
                uten_arbeidssoeker(
                    Meldingstype::Opplysninger,
                    o.periode_id,
                    Opphav::IKKE_AKTUELT,
                )
            }),
            Dekoder::Profilering => self.avro::<Profilering>(payload).await?.map(|p| {
                uten_arbeidssoeker(
                    Meldingstype::Profilering,
                    p.periode_id,
                    Opphav::IKKE_AKTUELT,
                )
            }),
            Dekoder::Egenvurdering => self.avro::<Egenvurdering>(payload).await?.map(|e| {
                uten_arbeidssoeker(
                    Meldingstype::Egenvurdering,
                    e.periode_id,
                    Opphav::IKKE_AKTUELT,
                )
            }),
            Dekoder::Bekreftelse => self.avro::<Bekreftelse>(payload).await?.map(|b| {
                uten_arbeidssoeker(
                    Meldingstype::Bekreftelse,
                    b.periode_id,
                    Opphav::bekreftelse(&b.bekreftelsesloesning),
                )
            }),
            Dekoder::PaaVegneAv => self.avro::<PaaVegneAv>(payload).await?.map(|p| {
                let meldingstype = match p.handling {
                    Handling::Start(_) => Meldingstype::PaaVegneAvStart,
                    Handling::Stopp(_) => Meldingstype::PaaVegneAvStopp,
                };
                uten_arbeidssoeker(meldingstype, p.periode_id, opphav_fra_header(msg))
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

fn uten_arbeidssoeker(
    meldingstype: Meldingstype,
    periode_id: Uuid,
    opphav: Opphav,
) -> DekodetMelding {
    DekodetMelding {
        meldingstype,
        periode_id: Some(periode_id),
        arbeidssoeker_id: None,
        opphav,
    }
}
