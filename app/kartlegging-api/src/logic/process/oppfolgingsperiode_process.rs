use crate::logic::mutation::kontortilknytning_mutation;
use dab_oppfolgingperioder::oppfolgingsperiode::Oppfolgingsperiode;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use sqlx::{Postgres, Transaction};

pub struct OppfolgingsperiodeProcessor;

impl OppfolgingsperiodeProcessor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: Oppfolgingsperiode = serde_json::from_slice(payload).map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;

        tracing::info!("Mottok hendelse: {:?}", &hendelse);
        kontortilknytning_mutation::lagre_hendelse(tx, &hendelse).await?;
        Ok(())
    }
}
