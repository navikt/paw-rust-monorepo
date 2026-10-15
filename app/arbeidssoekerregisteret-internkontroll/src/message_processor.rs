use std::pin::Pin;

use chrono::{DateTime, Utc};
use paw_kafka::{
    hwm::hwm_message_processor::{MessageProcessor, ProcessorError},
    signing::{RecordVerifier, VerificationError},
};
use paw_team_logs::TeamLogger;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};

pub struct InternkontrollMessageProcessor {
    record_verifier: RecordVerifier,
    team_logger: Box<dyn TeamLogger + Send + Sync>,
}

impl InternkontrollMessageProcessor {
    pub fn new(record_verifier: RecordVerifier, team_logger: impl TeamLogger + 'static) -> Self {
        InternkontrollMessageProcessor {
            record_verifier,
            team_logger: Box::new(team_logger),
        }
    }

    #[tracing::instrument(
    skip(self, _tx, msg),
    name = "paw_internkontroll.process",
    fields(
        topic = msg.topic(),
        partition = msg.partition(),
        offset = msg.offset(),
        timestamp = msg.timestamp().to_millis().unwrap_or(-1),
    )
)]
    pub async fn process(&self, _tx: &mut Transaction<'_, Postgres>, msg: &OwnedMessage) {
        let partition = msg.partition();
        let topic = msg.topic();
        let offset = msg.offset();
        let timstamp = msg.timestamp().to_millis();
        let _headers = msg.headers();
        let _key = msg.key();
        let _body = msg.payload();

        let reference_time: DateTime<Utc> =
            DateTime::from_timestamp_millis(timstamp.unwrap_or_default()).unwrap_or_else(Utc::now);
        let verification_result = self.record_verifier.validate(reference_time, msg);
        match verification_result {
            Ok(_) => {
                tracing::info!(
                    "Message {}-{}-{} verified successfully",
                    partition,
                    topic,
                    offset
                );
            }
            Err(VerificationError::SignatureMismatch) => {
                tracing::warn!(
                    "Message {}-{}-{} signature verification failed: cryptographic mismatch",
                    partition,
                    topic,
                    offset
                );
                let _ = self.team_logger.warn(
                    "arbeidssoekerregisteret-internkontroll",
                    &format!(
                        "Message {}-{}-{} signature verification failed: cryptographic mismatch",
                        partition, topic, offset
                    ),
                );
            }
            Err(e) => {
                tracing::error!(
                    "Message {}-{}-{} failed signature verification with error: {:?}",
                    partition,
                    topic,
                    offset,
                    e
                );
                let _ = self.team_logger.warn(
                    "arbeidssoekerregisteret-internkontroll",
                    &format!(
                        "Message {}-{}-{} failed signature verification with error: {:?}",
                        partition, topic, offset, e
                    ),
                );
            }
        }
        tracing::debug!(
            "Processed {}-{}-{} at {}",
            partition,
            topic,
            offset,
            timstamp.unwrap_or(-1)
        );
    }
}

impl MessageProcessor for InternkontrollMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(async move {
            self.process(tx, msg).await;
            Ok::<(), ProcessorError>(())
        })
    }
}
