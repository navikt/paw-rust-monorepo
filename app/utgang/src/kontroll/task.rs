use std::num::NonZeroU16;
use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::instrument;

use regler_arbeidssoeker::regler::regelsett::Regelsett;

#[derive(Clone)]
pub struct KontrollTask {
    inner: Arc<KontrollTaskInner>,
}

struct KontrollTaskInner {
    pg_pool: PgPool,
    batch_size: NonZeroU16,
    regelsett: Regelsett,
}

impl KontrollTask {
    pub fn new(pg_pool: PgPool, batch_size: NonZeroU16, regelsett: Regelsett) -> Self {
        Self {
            inner: Arc::new(KontrollTaskInner {
                pg_pool,
                batch_size,
                regelsett,
            }),
        }
    }

    #[instrument(skip(self))]
    pub async fn kjoer_kontroll(&self) -> Result<bool> {
        //let mut tx = self.inner.pg_pool.begin().await?;
        //tx.commit().await?;
        Ok(true)
    }
}

pub fn start_kontroll_task(
    kontroll: KontrollTask,
    intervall: std::time::Duration,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        loop {
            let hadde_arbeid = kontroll.kjoer_kontroll().await?;
            if !hadde_arbeid {
                sleep(intervall).await;
            }
        }
    })
}
