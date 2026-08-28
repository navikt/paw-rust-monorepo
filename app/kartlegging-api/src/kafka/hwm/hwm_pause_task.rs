use crate::config::AppConfig;
use crate::kafka::hwm::hwm_pause;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Sjekker periodisk om noen pausede partisjoner (avventer periode) har stått pauset lenger enn
/// `app_config.hwm_pause.stuck_partition_threshold` og flipper i så fall appens helsesjekk til
/// usunn. Det finnes ingen gi-opp-vei lenger — periode-topic har uendelig retention og periode
/// produseres alltid før korrelert bekreftelse, så partisjonen *skal* til slutt gjenopptas
/// naturlig når perioden ankommer. Terskelen er kun en operativ varslingsmekanisme: den flipper
/// helsesjekken slik at Nais/Kubernetes' liveness-probe (`/internal/isAlive`) feiler og gjør
/// avviket synlig (pod-restart/alarm), i stedet for at en fastlåst partisjon blir observert kun
/// via `paw_kafka_partition_paused`-gaugen. Se `kafka-synchronization-plan.md` og analysen i
/// sesjonsplanen for bakgrunn.
///
/// NB: en pod-restart løser ikke selve årsaken (pause-tilstanden er lagret i `hwm`-tabellen og
/// overlever restart), så dersom terskelen faktisk er nådd på grunn av et reelt
/// datakvalitetsproblem (brutt periode-før-bekreftelse-invariant) vil appen fortsette å
/// rapportere usunn og bli restartet gjentatte ganger helt til noen griper inn manuelt
/// (f.eks. oppdaterer `hwm`-raden eller undersøker hvorfor perioden mangler).
pub fn hwm_pause_timeout_task(
    app_config: Arc<AppConfig>,
    kafka_config: Arc<KafkaConfig>,
    pg_pool: PgPool,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    app_state: Arc<AppState>,
) -> JoinHandle<anyhow::Result<()>> {
    let hwm_version = *kafka_config.hwm_version;
    let stuck_partition_threshold = app_config.hwm_pause.stuck_partition_threshold;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(app_config.hwm_pause.check_interval);
        loop {
            interval.tick().await;
            let mut tx = match pg_pool.begin().await {
                Ok(tx) => tx,
                Err(e) => {
                    tracing::error!("Kunne ikke starte transaksjon for pause-sjekk: {e}");
                    continue;
                }
            };
            check_for_stuck_partitions(
                &mut tx,
                consumer.clone(),
                hwm_version,
                &app_state,
                stuck_partition_threshold,
            )
            .await;
            if let Err(e) = tx.commit().await {
                tracing::error!("Kunne ikke committe pause-sjekk-transaksjon: {e}");
            }
        }
    })
}

async fn check_for_stuck_partitions(
    tx: &mut Transaction<'_, Postgres>,
    consumer: Arc<StreamConsumer<HwmRebalanceHandler>>,
    version: i16,
    app_state: &AppState,
    threshold: Duration,
) {
    let paused = hwm_pause::paused_partitions(tx, consumer, version).await;
    let since: Vec<_> = paused.iter().map(|p| p.since).collect();

    if is_any_partition_stuck(&since, chrono::Utc::now(), threshold) {
        tracing::error!(
            "Partisjon(er) har vært pauset lenger enn {:?} — markerer appen som usunn",
            threshold
        );
        crate::logic::metrics::kafka_metrics::record_partition_stuck();
        app_state.set_is_alive(false);
    }
}

/// Avgjør om minst én pauset partisjon har passert `threshold`.
fn is_any_partition_stuck(
    paused_since: &[chrono::DateTime<chrono::Utc>],
    now: chrono::DateTime<chrono::Utc>,
    threshold: Duration,
) -> bool {
    paused_since.iter().any(|since| {
        let elapsed = (now - *since).to_std().unwrap_or(Duration::ZERO);
        elapsed >= threshold
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    const TERSKEL: Duration = Duration::from_secs(60 * 60);

    #[test]
    fn ingen_pausede_partisjoner_er_ikke_fastlaast() {
        let now = chrono::Utc::now();
        assert!(!is_any_partition_stuck(&[], now, TERSKEL));
    }

    #[test]
    fn ingen_partisjon_over_terskel_er_ikke_fastlaast() {
        let now = chrono::Utc::now();
        let nylig_pauset = now - ChronoDuration::minutes(5);
        let snart_over_grensen = now - ChronoDuration::minutes(59);

        assert!(!is_any_partition_stuck(
            &[nylig_pauset, snart_over_grensen],
            now,
            TERSKEL
        ));
    }

    #[test]
    fn en_partisjon_over_terskel_regnes_som_fastlaast() {
        let now = chrono::Utc::now();
        let nylig_pauset = now - ChronoDuration::minutes(5);
        let over_terskel = now - ChronoDuration::hours(2);

        assert!(is_any_partition_stuck(
            &[nylig_pauset, over_terskel],
            now,
            TERSKEL
        ));
    }

    #[test]
    fn partisjon_noyaktig_paa_terskelen_regnes_som_fastlaast() {
        let now = chrono::Utc::now();
        let noyaktig_paa_grensen = now - ChronoDuration::hours(1);

        assert!(is_any_partition_stuck(
            &[noyaktig_paa_grensen],
            now,
            TERSKEL
        ));
    }
}
