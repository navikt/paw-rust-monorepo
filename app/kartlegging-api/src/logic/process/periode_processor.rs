use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use pdl_client::pdl_query::PDLClient;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;

pub struct PeriodeProcessor {
    pub key_gen_client: Arc<PawKeyGenClient>,
    pub pdl_client: Arc<PDLClient>,
    pub deserializer: AvroDeserializer,
}

impl PeriodeProcessor {
    pub fn new(
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
        schema_registry_setting: SrSettings,
    ) -> Self {
        Self {
            key_gen_client,
            pdl_client,
            deserializer: AvroDeserializer::new(schema_registry_setting),
        }
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: Periode = self.deserializer.deserialize(payload).await.map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;
        self.handle_event(tx, &hendelse).await
    }

    async fn handle_event<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        hendelse: &'a Periode,
    ) -> anyhow::Result<(), ProcessorError> {
        tracing::info!("Mottok hendelse: {:?}", &hendelse);
        let identiteter_response = self
            .key_gen_client
            .finn_identiteter(hendelse.identitetsnummer.clone())
            .await?;
        tracing::info!("Fant identiteter: {:?}", &identiteter_response);
        Ok(())
    }
}
