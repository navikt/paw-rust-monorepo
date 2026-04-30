use crate::dao::perioder::{PeriodeRad, skriv_perioder};
use crate::dao::utgang_hendelse::{Input, InternUtgangHendelse};
use crate::dao::utgang_hendelser_logg::skriv_hendelser;
use crate::kafka::periode_processor::{PeriodeProcessor, PeriodeProcessorError};
use crate::kafka::schema_registry_config::create_schema_registry_settings;
use crate::{ARBEIDSSOKERPERIODER_TOPIC, HENDELSELOGG_TOPIC};
use anyhow::Result;
use interne_hendelser::InterneHendelser;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::Instrument;
use tracing::warn;

pub struct UtgangMessageProcessor {
    periode_processor: Arc<PeriodeProcessor>,
}

impl UtgangMessageProcessor {
    pub fn new() -> Result<UtgangMessageProcessor> {
        let sr_settings = create_schema_registry_settings()?;
        Ok(UtgangMessageProcessor {
            periode_processor: Arc::new(PeriodeProcessor::new(sr_settings)),
        })
    }
}

impl MessageProcessor for UtgangMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                let topic = msg.topic();
                match (topic, msg.payload()) {
                    (t, None) => {
                        return Err(PeriodeProcessorError::NoPayload {
                            topic: t.to_string(),
                            partition: msg.partition(),
                            offset: msg.offset(),
                        }
                        .into());
                    }
                    (t, Some(p)) if t == HENDELSELOGG_TOPIC => {
                        haandter_hendelse(tx, p).await?;
                    }
                    (t, Some(p)) if t == ARBEIDSSOKERPERIODER_TOPIC => {
                        haandter_periode_record(self, tx, p).await?;
                    }
                    _ => {
                        warn!("Received message for unknown topic: {}", topic);
                    }
                }
                Ok(())
            }
            .instrument(tracing::Span::current()),
        )
    }
}

async fn haandter_periode_record(
    utgang_message_processor: &UtgangMessageProcessor,
    tx: &mut Transaction<'_, Postgres>,
    payload: &[u8],
) -> Result<(), ProcessorError> {
    let periode = utgang_message_processor
        .periode_processor
        .deserialize_message(payload)
        .await?;
    let periode_rad: PeriodeRad = (&periode).into();
    let intern_utgang_hendelse: InternUtgangHendelse<Input> = periode.into();
    skriv_perioder(tx, vec![periode_rad]).await?;
    skriv_hendelser(tx, vec![intern_utgang_hendelse]).await?;
    Ok(())
}

async fn haandter_hendelse(
    tx: &mut Transaction<'_, Postgres>,
    payload: &[u8],
) -> Result<(), ProcessorError> {
    let payload_str = std::str::from_utf8(payload)
        .map_err(|e| ProcessorError::from(format!("Invalid UTF-8 in payload: {}", e)))?;

    let hendelse: InterneHendelser = serde_json::from_str(payload_str)
        .map_err(|e| ProcessorError::from(format!("Failed to deserialize event: {}", e)))?;
    match hendelse {
        InterneHendelser::Startet(startet) => {
            tracing::info!(
                "Mottok startet hendelse med hendelse_id: {}",
                startet.hendelse_id
            );
            let periode_rad: PeriodeRad = (&startet).into();
            let intern_utgang_hendelse: InternUtgangHendelse<Input> = startet.into();
            skriv_perioder(tx, vec![periode_rad]).await?;
            skriv_hendelser(tx, vec![intern_utgang_hendelse]).await?;
        }
        _ => {
            tracing::info!("Mottok en annen hendelse som ikke skal lagres");
        }
    }
    Ok(())
}
