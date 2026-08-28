use crate::kafka::hwm::hwm_pause;
use crate::model::error::PayloadProcessorError;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use paw_rdkafka_hwm::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use paw_rust_base::error::ServerError;
use rdkafka::Message;
use rdkafka::consumer::StreamConsumer;
use rdkafka::message::OwnedMessage;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn hwm_process_paused_partitions(
    pg_pool: PgPool,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    hwm_version: i16,
    message: &OwnedMessage,
    synced_topics: Vec<&str>,
    result: Result<(), ProcessorError>,
) -> anyhow::Result<()> {
    let mut tx = pg_pool.begin().await?;
    let topic = message.topic();
    let partition = message.partition();
    let offset = message.offset();

    match result {
        Ok(()) => {
            hwm_pause::mark_resolved(&mut tx, hwm_version, topic, partition).await;

            if should_attempt_resume(topic, &synced_topics)
                && let Err(e) = hwm_pause::resume_all(&mut tx, consumer, hwm_version).await
            {
                tracing::warn!("Kunne ikke gjenoppta pausede partisjoner: {e}");
            }

            tx.commit().await?;
            Ok(())
        }
        Err(e) => {
            if let Some(error) = e.downcast_ref::<PayloadProcessorError>() {
                match error {
                    PayloadProcessorError::AwaitingDependency { .. } => {
                        tracing::info!(
                            "Pauser partisjon {}:{} og venter på periode (offset {})",
                            topic,
                            partition,
                            offset
                        );
                        hwm_pause::pause_and_seek(
                            &mut tx,
                            consumer,
                            hwm_version,
                            topic,
                            partition,
                            offset,
                        )
                        .await
                        .map_err(|e| {
                            ServerError::InternalProcessTerminated {
                                process: "KafkaConsumer".to_string(),
                                message: format!("Kunne ikke pause/seek partisjon: {e}"),
                            }
                        })?;

                        tx.commit().await?;
                        Ok(())
                    }
                    _ => {
                        tx.rollback().await?;
                        Err(ServerError::InternalProcessTerminated {
                            process: "KafkaConsumer".to_string(),
                            message: e.to_string(),
                        }
                        .into())
                    }
                }
            } else {
                tx.rollback().await?;
                Err(ServerError::InternalProcessTerminated {
                    process: "KafkaConsumer".to_string(),
                    message: e.to_string(),
                }
                .into())
            }
        }
    }
}

/// Avgjør om en vellykket melding på `topic` bør trigge et forsøk på å gjenoppta alle pausede
/// partisjoner. Kun topics som andre topics faktisk kan avvente (i dag: `PAW_PERIODE_TOPIC`, se
/// `AppKafkaConfig::synced_topics`) trigger et resume-forsøk — å prøve resume for enhver
/// vellykket melding uansett topic ville vært et unødvendig, hyppigere DB/Kafka-kall uten å endre
/// utfallet (partisjoner som fortsatt avventer periode vil uansett bli pauset på nytt).
fn should_attempt_resume(topic: &str, synced_topics: &[&str]) -> bool {
    synced_topics.contains(&topic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periode_topic_trigger_resume_forsoek() {
        assert!(should_attempt_resume(
            "paw.arbeidssokerperioder-v1",
            &["paw.arbeidssokerperioder-v1"]
        ));
    }

    #[test]
    fn bekreftelse_topic_trigger_ikke_resume_forsoek() {
        assert!(!should_attempt_resume(
            "paw.arbeidssoker-bekreftelse-v1",
            &["paw.arbeidssokerperioder-v1"]
        ));
    }

    #[test]
    fn tom_synced_topics_liste_trigger_aldri_resume_forsoek() {
        assert!(!should_attempt_resume("paw.arbeidssokerperioder-v1", &[]));
    }
}
