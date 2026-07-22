use rdkafka::message::OwnedMessage;
use rdkafka::Timestamp;
use serde::Serialize;

pub struct JsonGenerator;

impl JsonGenerator {
    pub fn create_json_payload(&self, payload: impl Serialize) -> Vec<u8> {
        serde_json::to_vec(&payload).expect("Kunne ikke serialisere payload")
    }

    pub fn create_json_message(
        &self,
        topic: &'static str,
        payload: impl Serialize,
    ) -> OwnedMessage {
        let payload = self.create_json_payload(payload);
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
