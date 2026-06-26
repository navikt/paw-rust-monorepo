use crate::periode::Periode;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PeriodeDeserializerError {
    #[error("Failed to deserialize Avro message: {0}")]
    AvroDeserializationFailed(String),
    #[error("Avro deserialization error: {0}")]
    AvroError(#[from] apache_avro::Error),
}

pub struct PeriodeDeserializer {
    decoder: Arc<schema_registry_converter::async_impl::avro::AvroDecoder<'static>>,
}

impl PeriodeDeserializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let decoder =
            schema_registry_converter::async_impl::avro::AvroDecoder::new(schema_reg_settings);
        Self {
            decoder: Arc::new(decoder),
        }
    }

    pub async fn deserialize(&self, payload: &[u8]) -> Result<Periode, PeriodeDeserializerError> {
        // Decode using schema registry
        let decoded = self.decoder.decode(Some(payload)).await.map_err(|e| {
            PeriodeDeserializerError::AvroDeserializationFailed(format!(
                "Failed to decode Avro message with schema registry: {}",
                e
            ))
        })?;

        // Convert Avro value to Periode struct using apache_avro's built-in serde support
        let periode: Periode = apache_avro::from_value(&decoded.value)?;

        Ok(periode)
    }
}
