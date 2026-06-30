use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::{AvroDeserializer, AvroSerdeError};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::sync::Arc;

#[derive(Clone)]
pub struct PeriodeProcessor {
    deserializer: Arc<AvroDeserializer>,
}

impl PeriodeProcessor {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let deserializer = AvroDeserializer::new(schema_reg_settings);
        Self {
            deserializer: Arc::new(deserializer),
        }
    }

    pub async fn deserialize_message(&self, payload: &[u8]) -> Result<Periode, AvroSerdeError> {
        self.deserializer.deserialize(payload).await
    }
}
