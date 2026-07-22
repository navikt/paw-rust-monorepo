use eksterne_hendelser::serde::AvroSerializer;
use rdkafka::message::OwnedMessage;
use rdkafka::Timestamp;
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

    pub async fn create_avro_payload(
        &self,
        topic: &'static str,
        payload: impl Serialize,
    ) -> Vec<u8> {
        let strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), false);
        self.avro_serializer
            .serialize(payload, &strategy)
            .await
            .expect("Kunne ikke serialisere melding")
    }

    pub async fn create_avro_message(
        &self,
        topic: &'static str,
        payload: impl Serialize,
    ) -> OwnedMessage {
        let payload = self.create_avro_payload(topic, payload).await;
        OwnedMessage::new(
            Some(payload),
            Some("dummy-key".as_bytes().to_vec()),
            topic.to_string(),
            Timestamp::now(),
            0,
            0,
            None,
        )
    }
}
