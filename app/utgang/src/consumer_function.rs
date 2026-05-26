use crate::dao::skriv_periode::{skriv_periode_melding, skriv_startet_hendelse};
use crate::kafka::periode_processor::{PeriodeProcessor, PeriodeProcessorError};
use crate::kafka::schema_registry_config::create_schema_registry_settings;
use crate::{ARBEIDSSOKERPERIODER_TOPIC, HENDELSELOGG_TOPIC};
use anyhow::Result;
use interne_hendelser::{InterneHendelser, Startet};
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
                let res: Result<(), ProcessorError> = match (topic, msg.payload()) {
                    (t, None) => Err(PeriodeProcessorError::NoPayload {
                        topic: t.to_string(),
                        partition: msg.partition(),
                        offset: msg.offset(),
                    }
                    .into()),
                    (t, Some(p)) if t == HENDELSELOGG_TOPIC => {
                        async move {
                            if let Some(startet) = deserialize_startet_hendelse(p)? {
                                skriv_startet_hendelse(tx, startet).await?;
                            }
                            Ok(())
                        }
                        .await
                    }
                    (t, Some(p)) if t == ARBEIDSSOKERPERIODER_TOPIC => {
                        async move {
                            let periode = self.periode_processor.deserialize_message(p).await?;
                            skriv_periode_melding(tx, periode).await?;
                            Ok(())
                        }
                        .await
                    }
                    _ => {
                        warn!("Received message for unknown topic: {}", topic);
                        Ok(())
                    }
                };
                res
            }
            .instrument(tracing::Span::current()),
        )
    }
}

pub fn deserialize_startet_hendelse(payload: &[u8]) -> Result<Option<Startet>, ProcessorError> {
    let payload_str = std::str::from_utf8(payload)
        .map_err(|e| ProcessorError::from(format!("Invalid UTF-8 in payload: {}", e)))?;
    let hendelse: InterneHendelser = serde_json::from_str(payload_str)
        .map_err(|e| ProcessorError::from(format!("Failed to deserialize event: {}", e)))?;
    if let InterneHendelser::Startet(startet) = hendelse {
        Ok(Some(startet))
    } else {
        Ok(None)
    }
}
