use crate::model::result::ProcessorResult;
use paw_kafka::hwm::hwm_message_processor::ProcessorError;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};

pub mod root;
pub(crate) mod periode;

#[allow(async_fn_in_trait)]
pub trait PayloadProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<ProcessorResult, ProcessorError>;
}
