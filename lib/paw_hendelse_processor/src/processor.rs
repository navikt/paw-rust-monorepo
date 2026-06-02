use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::periode_serde::{PeriodeDeserializer, PeriodeDeserializerError};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::sync::Arc;

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

    pub async fn deserialize_message(
        &self,
        payload: &[u8],
    ) -> Result<Periode, PeriodeDeserializerError> {
        self.deserializer.deserialize(payload).await
    }
}
