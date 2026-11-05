//! Kafka-meldinger med ekte innhold. Oppgi det testen bryr seg om, resten får standardverdier:
//!
//! ```ignore
//! app.les_inn(StartetBuilder::default()).await;
//! app.les_inn(Melding::ny(StartetBuilder { arbeidssoeker_id: 42, ..Default::default() })
//!     .offset(3)
//!     .tidspunkt_ms(1_000))
//!     .await;
//! ```

use eksterne_hendelser::periode::Periode;
use interne_hendelser::InterneHendelser;
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::StartetBuilder;
use rdkafka::Timestamp;
use rdkafka::message::{Header, OwnedHeaders, OwnedMessage};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use test_data_generator::avro::AvroGenerator;

use super::topic_navn;

/// Innhold som hører til et bestemt topic. `payload` er asynkron fordi avro trenger schema registry.
pub trait Innhold {
    fn topic(&self) -> Topic;
    fn payload(&self, sr: &SrSettings) -> impl Future<Output = Vec<u8>>;
}

impl Innhold for StartetBuilder {
    fn topic(&self) -> Topic {
        Topic::Hendelselogg
    }
    async fn payload(&self, _: &SrSettings) -> Vec<u8> {
        serde_json::to_vec(&InterneHendelser::Startet(self.build())).unwrap()
    }
}

/// Lag med `create_dummy_start_periode` eller `create_dummy_avslutt_periode`.
impl Innhold for Periode {
    fn topic(&self) -> Topic {
        Topic::Periode
    }
    async fn payload(&self, sr: &SrSettings) -> Vec<u8> {
        AvroGenerator::new(sr.clone())
            .create_avro_payload(topic_navn(&self.topic()), self.clone())
            .await
    }
}

/// 2024-01-01T12:00:00Z, samme som standardtidspunktet i `StartetBuilder`.
const STANDARD_TIDSPUNKT_MS: i64 = 1_704_110_400_000;

pub struct Melding<I> {
    pub innhold: I,
    pub partisjon: i32,
    /// `None`: neste ledige offset i topic-partisjonen.
    pub offset: Option<i64>,
    pub tidspunkt_ms: i64,
    pub headers: Vec<(String, Vec<u8>)>,
}

impl<I: Innhold> Melding<I> {
    pub fn ny(innhold: I) -> Self {
        Self {
            innhold,
            partisjon: 0,
            offset: None,
            tidspunkt_ms: STANDARD_TIDSPUNKT_MS,
            headers: Vec::new(),
        }
    }

    pub fn partisjon(self, partisjon: i32) -> Self {
        Self { partisjon, ..self }
    }

    pub fn offset(self, offset: i64) -> Self {
        Self {
            offset: Some(offset),
            ..self
        }
    }

    pub fn tidspunkt_ms(self, tidspunkt_ms: i64) -> Self {
        Self {
            tidspunkt_ms,
            ..self
        }
    }

    pub fn header(mut self, key: &str, value: impl Into<Vec<u8>>) -> Self {
        self.headers.push((key.to_string(), value.into()));
        self
    }

    pub(super) async fn bygg(&self, offset: i64, sr: &SrSettings) -> OwnedMessage {
        let headers = self
            .headers
            .iter()
            .fold(OwnedHeaders::new(), |headers, (key, value)| {
                headers.insert(Header {
                    key,
                    value: Some(value),
                })
            });
        OwnedMessage::new(
            Some(self.innhold.payload(sr).await),
            None,
            topic_navn(&self.innhold.topic()).into(),
            Timestamp::CreateTime(self.tidspunkt_ms),
            self.partisjon,
            offset,
            Some(headers),
        )
    }
}

impl<I: Innhold> From<I> for Melding<I> {
    fn from(innhold: I) -> Self {
        Self::ny(innhold)
    }
}
