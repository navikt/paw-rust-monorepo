use schema_registry_converter::async_impl::schema_registry::SrSettings;
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AvroSerdeError {
    #[error("Failed to serialize Avro message: {0}")]
    AvroSerializationFailed(String),
    #[error("Failed to deserialize Avro message: {0}")]
    AvroDeserializationFailed(String),
    #[error("Avro deserialization error: {0}")]
    AvroError(#[from] apache_avro::Error),
}

pub struct AvroDeserializer {
    decoder: Arc<schema_registry_converter::async_impl::avro::AvroDecoder<'static>>,
}

impl AvroDeserializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let decoder =
            schema_registry_converter::async_impl::avro::AvroDecoder::new(schema_reg_settings);
        Self {
            decoder: Arc::new(decoder),
        }
    }

    pub async fn deserialize<D: DeserializeOwned>(
        &self,
        payload: &[u8],
    ) -> Result<D, AvroSerdeError> {
        let decoded = self.decoder.decode(Some(payload)).await.map_err(|e| {
            AvroSerdeError::AvroDeserializationFailed(format!(
                "Failed to deserialize Avro message with schema registry: {}",
                e
            ))
        })?;

        let avro: D = apache_avro::from_value(&decoded.value)?;
        Ok(avro)
    }
}

pub struct AvroSerializer {
    encoder: Arc<schema_registry_converter::async_impl::avro::AvroEncoder<'static>>,
}

impl AvroSerializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let encoder =
            schema_registry_converter::async_impl::avro::AvroEncoder::new(schema_reg_settings);
        Self {
            encoder: Arc::new(encoder),
        }
    }

    pub async fn serialize(
        &self,
        avro: impl Serialize,
        strategy: &SubjectNameStrategy,
    ) -> Result<Vec<u8>, AvroSerdeError> {
        let payload = self
            .encoder
            .encode_struct(avro, strategy)
            .await
            .map_err(|e| {
                AvroSerdeError::AvroSerializationFailed(format!(
                    "Failed to serialize Avro message with schema registry: {}",
                    e
                ))
            })?;

        Ok(payload)
    }
}
