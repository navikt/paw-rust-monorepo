use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{PgPool, Postgres, Transaction};

pub struct PeriodeProcessor {
    pub pg_pool: PgPool,
    pub deserializer: AvroDeserializer,
}

impl PeriodeProcessor {
    pub fn new(pg_pool: PgPool, schema_registry_setting: SrSettings) -> Self {
        Self {
            pg_pool,
            deserializer: AvroDeserializer::new(schema_registry_setting),
        }
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let periode: Periode = self.deserializer.deserialize(payload).await.map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;
        tracing::info!(
            "Mottok arbeidssokerperiode: {}",
            serde_json::to_string(&periode)
                .unwrap_or_else(|_| "Failed to serialize periode".to_string())
        );
        Ok(())
    }
}
