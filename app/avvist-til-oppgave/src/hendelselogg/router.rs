use super::avvist_under_18::opprett_oppgave_avvist_under_18;
use super::vurder_opphold::opprett_oppgave_vurder_opphold;
use crate::config::ApplicationConfig;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use serde_json::Value;
use sqlx::{Postgres, Transaction};

pub async fn process_hendelselogg_message(
    kafka_message: &OwnedMessage,
    app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let payload = kafka_message.payload().unwrap_or(&[]);
    let hendelse_json: Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(_) => {
            tracing::warn!(
                "Klarte ikke å deserialisere Kafka-melding fra hendelselogg som JSON, hopper over"
            );
            return Ok(());
        }
    };
    let hendelse_type = hendelse_json["hendelseType"].as_str().unwrap_or_default();

    match hendelse_type {
        interne_hendelser::AVVIST_HENDELSE_TYPE => {
            opprett_oppgave_avvist_under_18(hendelse_json, app_config, tx).await?;
        }
        interne_hendelser::STARTET_HENDELSE_TYPE => {
            //opprett_oppgave_vurder_opphold(hendelse_json, tx).await?;
        }
        _ => {}
    }

    Ok(())
}
