use crate::database::insert_data::insert_data;
use crate::kafka::headers::extract_headers_as_json;
use chrono::DateTime;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;
use tracing::Instrument;

pub struct BackupMessageProcessor;

impl MessageProcessor for BackupMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move { lagre_melding(msg, tx).await }.instrument(tracing::Span::current()),
        )
    }
}

pub async fn lagre_melding(
    msg: &OwnedMessage,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), ProcessorError> {
    let timestamp_millis = msg.timestamp().to_millis().unwrap_or(0);
    let timestamp = DateTime::from_timestamp_millis(timestamp_millis)
        .ok_or_else(|| format!("Invalid timestamp: {timestamp_millis}"))?;
    insert_data(
        tx,
        msg.topic(),
        msg.partition(),
        msg.offset(),
        timestamp,
        extract_headers_as_json(msg),
        msg.key().unwrap_or(&[]).to_vec(),
        msg.payload().unwrap_or(&[]).to_vec(),
    )
    .await?;
    Ok(())
}
