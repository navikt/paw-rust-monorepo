use std::{num::NonZeroU16, sync::Arc};

use crate::pdl::pdl_query::PDLClient;
use anyhow::Result;
use chrono::Duration;
use sqlx::PgPool;
use tracing::instrument;

#[derive(Clone)]
pub struct PdlDataOppdatering {
    inner: Arc<PdlDataOppdateringRef>,
}

struct PdlDataOppdateringRef {
    pg_pool: PgPool,
    pdl_client: PDLClient,
    batch_size: NonZeroU16,
    intervall: Duration,
}

impl PdlDataOppdatering {
    pub fn new(
        pg_pool: PgPool,
        pdl_client: PDLClient,
        batch_size: NonZeroU16,
        intervall: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(PdlDataOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
                intervall,
            }),
        }
    }
    #[instrument(skip(self))]
    pub async fn kjoer_oppdatering(&self) -> Result<()> {
        tracing::info!("Starter oppdatering av PDL data");
        Ok(())
    }
}
