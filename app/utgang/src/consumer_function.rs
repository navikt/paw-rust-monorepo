use crate::db_ops::skrive_startet_hendelse;
use crate::kafka::hwm_message_processor::{MessageProcessor, ProcessorError};
use crate::kafka::periode_processor::PeriodeProcessor;
use crate::kafka::schema_registry_config::create_schema_registry_settings;
use crate::{ARBEIDSSOKERPERIODER_TOPIC, HENDELSELOGG_TOPIC};
use anyhow::Result;
use interne_hendelser::InterneHendelser;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
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
        Box::pin(async move {
            let topic = msg.topic();
            match topic {
                t if t == HENDELSELOGG_TOPIC => {
                    // Get the payload as bytes
                    let payload = msg
                        .payload()
                        .ok_or_else(|| ProcessorError::from("Message has no payload"))?;

                    // Convert bytes to UTF-8 string
                    let payload_str = std::str::from_utf8(payload).map_err(|e| {
                        ProcessorError::from(format!("Invalid UTF-8 in payload: {}", e))
                    })?;

                    // Deserialize JSON string to InterneHendelser
                    let hendelse: InterneHendelser =
                        serde_json::from_str(payload_str).map_err(|e| {
                            ProcessorError::from(format!("Failed to deserialize event: {}", e))
                        })?;
                    match hendelse {
                        InterneHendelser::Startet(startet) => {
                            let key = msg
                                .key()
                                .and_then(|bytes| {
                                    if bytes.len() == 8 {
                                        Some(i64::from_be_bytes(bytes.try_into().unwrap()))
                                    } else {
                                        None
                                    }
                                })
                                .ok_or_else(|| {
                                    ProcessorError::from("Message key is missing or invalid")
                                })?;
                            tracing::info!(
                                "Mottok startet hendelse med hendelse_id: {}, record_key: {}",
                                startet.hendelse_id,
                                key
                            );
                            skrive_startet_hendelse(tx, &startet, key).await?;
                        }
                        _ => {
                            tracing::info!("Mottok en annen hendelse som ikke skal lagres");
                        }
                    }
                }
                t if t == ARBEIDSSOKERPERIODER_TOPIC => {
                    let key = msg
                        .key()
                        .and_then(|bytes| {
                            if bytes.len() == 8 {
                                Some(i64::from_be_bytes(bytes.try_into().unwrap()))
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| ProcessorError::from("Message key is missing or invalid"))?;
                }
                _ => {
                    warn!("Received message for unknown topic: {}", topic);
                }
            }
            Ok(())
        })
    }
}
