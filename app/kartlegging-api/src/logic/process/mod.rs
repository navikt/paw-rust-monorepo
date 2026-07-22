use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};

pub(crate) mod bekreftelse_paavegneav_process;
pub(crate) mod bekreftelse_process;
pub(crate) mod egenvurdering_process;
pub mod message_process;
pub(crate) mod oppfolgingsperiode_process;
pub(crate) mod opplysninger_process;
pub(crate) mod periode_process;
pub(crate) mod profilering_process;

#[allow(async_fn_in_trait)]
pub trait PayloadProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<(), ProcessorError>;
}
