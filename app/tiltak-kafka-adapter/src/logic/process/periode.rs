use crate::logic::process::PayloadProcessor;
use crate::model::dao::arbeidssoeker;
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::error::PayloadProcessorError;
use crate::model::result::ProcessorResult;
use eksterne_hendelser::periode::Periode;
use paw_avro_serde::serde::AvroDeserializer;
use paw_kafka::hwm::hwm_message_processor::ProcessorError;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};

pub struct PeriodeProcessor {
    pub deserializer: AvroDeserializer,
}

impl PeriodeProcessor {
    pub fn new(schema_registry_settings: SrSettings) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_settings),
        }
    }
}

impl PayloadProcessor for PeriodeProcessor {
    #[tracing::instrument(skip_all, fields(topic = %message.topic(), partition = %message.partition(), offset = %message.offset()
    ))]
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<ProcessorResult, ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let periode: Periode = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok {}-hendelse", &periode);

                if periode.avsluttet.is_none() {
                    let count = arbeidssoeker::count_by_periode_id(tx, &periode.id).await?;
                    if count > 0 {
                        let row = ArbeidssoekerRow::from(periode);
                        let rows_affected = arbeidssoeker::update(tx, &row).await?;
                        tracing::debug!(
                            "Opprettet arbeidssøker-rad ({} rows affected)",
                            rows_affected
                        );
                    } else {
                        let row = ArbeidssoekerRow::from(periode);
                        let rows_affected = arbeidssoeker::insert(tx, &row).await?;
                        tracing::debug!(
                            "Opprettet arbeidssøker-rad ({} rows affected)",
                            rows_affected
                        );
                    }
                } else {
                    let rows_affected =
                        arbeidssoeker::delete_by_periode_id(tx, &periode.id).await?;
                    tracing::debug!("Slettet arbeidssøker-rad ({} rows affected)", rows_affected);
                }

                Ok(ProcessorResult::Continue)
            }
        }
    }
}

impl From<Periode> for ArbeidssoekerRow {
    fn from(periode: Periode) -> Self {
        Self {
            periode_id: periode.id,
            identitetsnummer: periode.identitetsnummer,
            periode_startet: periode.startet.tidspunkt,
        }
    }
}

#[cfg(test)]
mod tests {}
