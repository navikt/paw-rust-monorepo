use std::pin::Pin;

use paw_kafka::hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};

pub struct InternkontrollMessageProcessor {}

impl MessageProcessor for InternkontrollMessageProcessor {
    fn process_message<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(async move {
            process(msg).await;
            Ok::<(), ProcessorError>(())
        })
    }
}

#[tracing::instrument(
    skip(msg),
    name = "paw_internkontroll.process",
    fields(
        topic = msg.topic(),
        partition = msg.partition(),
        offset = msg.offset(),
        timestamp = msg.timestamp().to_millis().unwrap_or(-1),
    )
)]
pub async fn process(msg: &OwnedMessage) {
    let partition = msg.partition();
    let topic = msg.topic();
    let offset = msg.offset();
    let timstamp = msg.timestamp().to_millis();
    tracing::debug!(
        "Processed {}-{}-{} at {}",
        partition,
        topic,
        offset,
        timstamp.unwrap_or(-1)
    );
}
