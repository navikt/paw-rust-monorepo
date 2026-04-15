use crate::config::ApplicationConfig;
use crate::hendelselogg::process_hendelselogg_message;
use crate::process_oppgavehendelse_message::oppdater_ferdigstilte_oppgaver;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use tracing::Instrument;

pub struct AvvistTilOppgaveMessageProcessor {
    pub app_config: ApplicationConfig,
}

impl MessageProcessor for AvvistTilOppgaveMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                let topic = msg.topic();
                let hendelseslogg_topic = &self.app_config.topic_hendelseslogg;
                let oppgavehendelse_topic = &self.app_config.topic_oppgavehendelse;

                if topic == hendelseslogg_topic.as_str() {
                    process_hendelselogg_message(msg, &self.app_config, tx).await?;
                } else if topic == oppgavehendelse_topic.as_str() {
                    oppdater_ferdigstilte_oppgaver(msg, *self.app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt, tx).await?;
                } else {
                    tracing::warn!("Mottok melding fra uventet topic: {}", topic);
                }
                Ok(())
            }
            .instrument(tracing::Span::current()),
        )
    }
}
