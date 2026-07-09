use crate::logic::mutation::egenvurdering_mutation;
use eksterne_hendelser::egenvurdering::Egenvurdering;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};

pub struct EgenvurderingProcessor {
    pub deserializer: AvroDeserializer,
}

impl EgenvurderingProcessor {
    pub fn new(schema_registry_setting: SrSettings) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_setting),
        }
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: Egenvurdering =
            self.deserializer.deserialize(payload).await.map_err(|e| {
                ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
            })?;

        tracing::info!("Mottok hendelse: {:?}", &hendelse);
        egenvurdering_mutation::lagre_hendelse(tx, &hendelse).await?;
        Ok(())
    }
}
