use crate::kafka::periode_deserializer::{Periode, PeriodeDeserializer};
use rdkafka::{message::OwnedMessage, Message};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone)]
pub struct PeriodeProcessor {
    deserializer: Arc<PeriodeDeserializer>,
}

impl PeriodeProcessor {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let deserializer = PeriodeDeserializer::new(schema_reg_settings);
        Self {
            deserializer: Arc::new(deserializer),
        }
    }

    pub async fn deserialize_message(&self, msg: &OwnedMessage) -> Result<Periode, PeriodeProcessorError> {
        let payload = msg
            .payload()
            .ok_or_else(|| PeriodeProcessorError::NoPayload {
                topic: msg.topic().to_string(),
                partition: msg.partition(),
                offset: msg.offset(),
            })?;

        let periode = self.deserializer.deserialize(payload).await
            .map_err(|e| PeriodeProcessorError::DeserializationError {
                message: e.to_string(),
                topic: msg.topic().to_string(),
                partition: msg.partition(),
                offset: msg.offset(),
            })?;

        Ok(periode)
    }
}

#[derive(Error, Debug)]
pub enum PeriodeProcessorError {
    #[error("Failed to deserialize payload from topic '{topic}' at partition {partition}, offset {offset}: {message}")]
    DeserializationError {
        message: String,
        topic: String,
        partition: i32,
        offset: i64
    },
    #[error("Message has no payload from topic '{topic}' at partition {partition}, offset {offset}")]
    NoPayload {
        topic: String,
        partition: i32,
        offset: i64
    },
    #[error("Processing failed for message from topic '{topic}' at partition {partition}, offset {offset}: {message}")]
    ProcessingError {
        message: String,
        topic: String,
        partition: i32,
        offset: i64
    }
}
