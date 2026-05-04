use crate::config::ApplicationConfig;
use crate::opprettelse::process_hendelselogg_message;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use tracing::Instrument;
use crate::ferdigstilling::ferdigstill_oppgave::ferdigstill_oppgave;

pub struct VeilederOppgaveMessageProcessor {
    pub app_config: ApplicationConfig,
}

impl MessageProcessor for VeilederOppgaveMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                let topic = msg.topic();
                let kafka_message_payload = msg.payload().unwrap_or(&[]);
                let hendelseslogg_topic = &self.app_config.topic_hendelseslogg;
                let oppgavehendelse_topic = &self.app_config.topic_oppgavehendelse;

                if topic == hendelseslogg_topic.as_str() {
                    process_hendelselogg_message(kafka_message_payload, &self.app_config, tx).await?;
                } else if topic == oppgavehendelse_topic.as_str() {
                    ferdigstill_oppgave(kafka_message_payload, tx).await?;
                } else {
                    tracing::warn!("Mottok melding fra uventet topic: {}", topic);
                }
                Ok(())
            }
            .instrument(tracing::Span::current()),
        )
    }
}
