use crate::config::AppConfig;
use crate::logic::process::PayloadProcessor;
use crate::logic::process::periode::PeriodeProcessor;
use crate::model::result::ProcessorResult;
use paw_kafka::hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::pin::Pin;
use std::sync::Arc;
use tracing::Instrument;

pub struct RootProcessor {
    app_config: Arc<AppConfig>,
    periode_processor: Arc<PeriodeProcessor>,
}

impl RootProcessor {
    pub fn new(
        app_config: Arc<AppConfig>,
        schema_registry_settings: SrSettings,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            app_config: app_config.clone(),
            periode_processor: Arc::new(PeriodeProcessor::new(schema_registry_settings.clone())),
        })
    }
}

impl MessageProcessor for RootProcessor {
    #[tracing::instrument(skip_all, fields(topic = %message.topic(), partition = %message.partition(), offset = %message.offset()))]
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                tracing::debug!(
                    "Mottok melding på topic: {}, partition: {}, offset: {}",
                    message.topic(),
                    message.partition(),
                    message.offset()
                );

                let kafka_config = &self.app_config.kafka;

                let result = match message.topic() {
                    topic if topic == kafka_config.paw_periode_topic.as_str() => {
                        self.periode_processor.process_payload(tx, message).await
                    }
                    topic => {
                        panic!("Mottok melding på ukjent topic: {}", topic);
                    }
                };

                match result {
                    Ok(ProcessorResult::Continue) => {
                        tracing::debug!(
                            "Prosessering av melding på topic: {}, partition: {}, offset: {} fullførte",
                            message.topic(),
                            message.partition(),
                            message.offset()
                        );
                        Ok(())
                    },
                    Err(e) => {
                        tracing::error!(
                            error = e,
                            "Prosessering av melding på topic: {}, partition: {}, offset: {} feilet",
                            message.topic(),
                            message.partition(),
                            message.offset()
                        );
                        Err(e)
                    }
                }
            }
            .instrument(tracing::Span::current()),
        )
    }
}

#[cfg(test)]
mod tests {}
