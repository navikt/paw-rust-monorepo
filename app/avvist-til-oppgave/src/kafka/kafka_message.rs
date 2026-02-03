use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use rdkafka::Message;
use rdkafka::message::BorrowedMessage;
use rdkafka::message::Headers;
use serde_json::{Map, Value};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub headers: Option<Value>,
    pub key: Vec<u8>,
    pub payload: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

impl KafkaMessage {
    pub fn from_borrowed_message(msg: BorrowedMessage<'_>) -> Result<Self, Box<dyn Error>> {
        let timestamp_millis = msg.timestamp().to_millis().unwrap_or(0);
        let timestamp = DateTime::from_timestamp_millis(timestamp_millis)
            .ok_or_else(|| format!("Invalid timestamp: {}", timestamp_millis))?;

        Ok(KafkaMessage {
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
            headers: extract_headers_as_json(&msg)?,
            key: msg.key().unwrap_or(&[]).to_vec(),
            payload: msg.payload().unwrap_or(&[]).to_vec(),
            timestamp,
        })
    }
}

fn extract_headers_as_json(msg: &BorrowedMessage<'_>) -> Result<Option<Value>, Box<dyn Error>> {
    match msg.headers() {
        Some(headers) => {
            let mut header_map = Map::new();

            for header in headers.iter() {
                let key = header.key;
                let value = match header.value {
                    Some(val) => {
                        // Try to decode as UTF-8, fallback to base64 for binary data
                        match std::str::from_utf8(val) {
                            Ok(s) => Value::String(s.to_string()),
                            Err(_) => Value::String(general_purpose::STANDARD.encode(val)),
                        }
                    }
                    None => Value::Null,
                };
                header_map.insert(key.to_string(), value);
            }

            Ok(Some(serde_json::to_value(header_map)?))
        }
        None => Ok(None),
    }
}
