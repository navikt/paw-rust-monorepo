use std::{num::NonZeroU16, sync::Arc};

use crate::pdl::pdl_query::PDLClient;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
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
    data_gyldighet: Duration,
}

impl PdlDataOppdatering {
    pub fn new(
        pg_pool: PgPool,
        pdl_client: PDLClient,
        batch_size: NonZeroU16,
        intervall: Duration,
        data_gyldighet: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(PdlDataOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
                intervall,
                data_gyldighet,
            }),
        }
    }
    #[instrument(skip(self))]
    pub async fn kjoer_oppdatering(&self, vannmerke: DateTime<Utc>) -> Result<()> {
        tracing::info!("Starter oppdatering av PDL data");
        Ok(())
    }
}
