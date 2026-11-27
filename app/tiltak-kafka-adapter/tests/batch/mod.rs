use mulighetsrommet::tiltakshistorikk::TiltakshistorikkV1Response;
use sqlx::PgPool;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tiltak_kafka_adapter::kafka::publisher::TiltakshistorikkPublisher;
use tiltak_kafka_adapter::logic::batch::BatchBehandler;
use tiltak_kafka_adapter::model::dao::arbeidssoeker;
use tiltak_kafka_adapter::model::dao::tiltak_synk::BatchTiltakSynkRow;
use uuid::Uuid;

mod tiltak_synk_batch;

struct TestBehandler<F>(F);

impl<F> BatchBehandler for TestBehandler<F>
where
    F: FnMut(Vec<BatchTiltakSynkRow>) -> ControlFlow<()> + Send,
{
    async fn behandle(&mut self, rows: Vec<BatchTiltakSynkRow>) -> ControlFlow<()> {
        (self.0)(rows)
    }
}

struct TestPublisher {
    kall: AtomicUsize,
    slett_periode: Option<PgPool>,
}

impl TestPublisher {
    fn new(slett_periode: Option<PgPool>) -> Arc<Self> {
        Arc::new(Self {
            kall: AtomicUsize::new(0),
            slett_periode,
        })
    }
}

impl TiltakshistorikkPublisher for TestPublisher {
    async fn publiser(
        &self,
        periode_id: &Uuid,
        _historikk: &TiltakshistorikkV1Response,
    ) -> anyhow::Result<()> {
        self.kall.fetch_add(1, Ordering::SeqCst);
        if let Some(pool) = &self.slett_periode {
            let mut tx = pool.begin().await?;
            arbeidssoeker::delete_by_periode_id(&mut tx, periode_id).await?;
            tx.commit().await?;
        }
        Ok(())
    }
}
