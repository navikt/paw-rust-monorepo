use crate::kafka::error::OversiktProcessorError;
use crate::logic::process::periode_processor::PeriodeProcessor;
use eksterne_hendelser::periode::PERIODE_TOPIC;
use nais_schema_registry::config::create_schema_registry_settings;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use sqlx::{PgPool, Postgres, Transaction};
use std::pin::Pin;
use std::sync::Arc;
use tracing::{warn, Instrument};

pub trait MessageProcessorTrait {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>>;
}

pub struct KartleggingMessageProcessor {
    periode_processor: Arc<PeriodeProcessor>,
}

impl KartleggingMessageProcessor {
    pub fn new(pg_pool: PgPool) -> anyhow::Result<Self> {
        let schema_registry_settings = create_schema_registry_settings()?;
        Ok(Self {
            periode_processor: Arc::new(PeriodeProcessor::new(pg_pool, schema_registry_settings)),
        })
    }
}

impl MessageProcessor for KartleggingMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                let res: anyhow::Result<(), ProcessorError> =
                    match (message.topic(), message.payload()) {
                        (topic, None) => Err(OversiktProcessorError::NoPayload {
                            topic: topic.to_string(),
                            partition: message.partition(),
                            offset: message.offset(),
                        }
                        .into()),
                        (topic, Some(payload)) if topic == PERIODE_TOPIC => {
                            self.periode_processor.process_payload(tx, payload).await
                        }
                        (topic, _) => {
                            warn!("Mottok melding på ukjent topic: {}", topic);
                            Ok(())
                        }
                    };
                res
            }
            .instrument(tracing::Span::current()),
        )
    }
}
