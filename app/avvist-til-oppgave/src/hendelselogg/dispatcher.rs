use crate::config::ApplicationConfig;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use serde_json::Value;
use sqlx::{Postgres, Transaction};

pub async fn process_hendelselogg_message(
    kafka_message: &OwnedMessage,
    app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let payload = kafka_message.payload().unwrap_or(&[]);
    let json: Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(_) => {
            tracing::warn!(
                "Klarte ikke å deserialisere Kafka-melding fra hendelselogg som JSON, hopper over"
            );
            return Ok(());
        }
    };
    let hendelse_type = json["hendelseType"].as_str().unwrap_or_default();

    match hendelse_type {
        interne_hendelser::AVVIST_HENDELSE_TYPE => {
            super::avvist_under_18::opprett_oppgave_for_avvist_hendelse(json, app_config, tx)
                .await?;
        }
        interne_hendelser::STARTET_HENDELSE_TYPE => {
            super::startet_eu_eoes_ikke_bosatt::opprett_oppgave_for_startet_hendelse(json, tx)
                .await?;
        }
        _ => {}
    }

    Ok(())
}
