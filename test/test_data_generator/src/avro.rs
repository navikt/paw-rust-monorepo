use paw_avro_serde::serde::AvroSerializer;
use rdkafka::Timestamp;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use serde::Serialize;

pub struct AvroGenerator {
    avro_serializer: AvroSerializer,
}

impl AvroGenerator {
    pub fn new(schema_registry_settings: SrSettings) -> Self {
        Self {
            avro_serializer: AvroSerializer::new(schema_registry_settings),
        }
    }

    pub async fn create_avro_key<K: Serialize>(&self, topic: &'static str, key: K) -> Vec<u8> {
        let strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), true);
        self.avro_serializer
            .serialize(key, &strategy)
            .await
            .expect("Kunne ikke serialisere melding")
    }

    pub async fn create_avro_payload<V: Serialize>(
        &self,
        topic: &'static str,
        payload: V,
    ) -> Vec<u8> {
        let strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), false);
        self.avro_serializer
            .serialize(payload, &strategy)
            .await
            .expect("Kunne ikke serialisere melding")
    }

    pub async fn create_avro_message<V: Serialize>(
        &self,
        topic: &'static str,
        key: Option<Vec<u8>>,
        payload: Option<V>,
    ) -> OwnedMessage {
        let avro_payload = match payload {
            Some(p) => Some(self.create_avro_payload(topic, p).await),
            None => None,
        };
        OwnedMessage::new(
            avro_payload,
            key,
            topic.to_string(),
            Timestamp::now(),
            0,
            0,
            None,
        )
    }
}
