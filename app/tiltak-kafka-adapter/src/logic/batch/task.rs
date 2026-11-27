use crate::kafka::publisher::TiltakshistorikkPublisher;
use crate::logic::batch::tiltak_synk::TiltakSynkJobb;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

pub fn tiltak_synk_task<P>(jobb: Arc<TiltakSynkJobb<P>>) -> JoinHandle<anyhow::Result<()>>
where
    P: TiltakshistorikkPublisher + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let config = jobb.config();
        if !config.aktivert {
            tracing::info!("Synkronisering av tiltakshistorikk er deaktivert");
            return std::future::pending().await;
        } else {
            tracing::info!("Synkronisering av tiltakshistorikk er aktiv");
            let mut interval = tokio::time::interval(config.task_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            interval.tick().await; // Vent ett interval før oppstart

            loop {
                interval.tick().await;
                jobb.run(Uuid::new_v4()).await;
            }
        }
    })
}
