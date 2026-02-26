use std::{num::NonZeroU16, ops::Deref, sync::Arc};

use crate::{
    db_read_ops::{hent_opplysninger, hent_periode_metadata, hent_sist_oppdatert_foer},
    pdl::pdl_query::PDLClient,
};
use anyhow::Result;
use sqlx::PgPool;

#[derive(Clone)]
pub struct StatusOppdatering {
    inner: Arc<StatusOppdateringRef>,
}

struct StatusOppdateringRef {
    pg_pool: PgPool,
    pdl_client: PDLClient,
    batch_size: NonZeroU16,
}

impl StatusOppdatering {
    pub fn new(pg_pool: PgPool, pdl_client: PDLClient, batch_size: NonZeroU16) -> Self {
        Self {
            inner: Arc::new(StatusOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
            }),
        }
    }

    pub async fn kjoer_oppdatering(&self) -> Result<()> {
        let pg_pool = &self.inner.pg_pool;
        let pdl_client = &self.inner.pdl_client;
        let batch_size = &self.inner.batch_size;
        let mut tx = pg_pool.begin().await?;
        let skal_oppdateres =
            hent_sist_oppdatert_foer(&mut tx, &chrono::Utc::now(), batch_size).await?;
        tx.commit().await?;

        for periode in skal_oppdateres {
            let mut tx = pg_pool.begin().await?;
            let periode_metadata = hent_periode_metadata(&mut tx, &periode.id).await?;
            let siste_opplysninger = hent_opplysninger(&mut tx, &periode.id, 1).await?;
            let a: Vec<String> = vec![periode_metadata.identitetsnummer];
            let gjeldene_pdl_data = pdl_client.perform_hent_person_bolk(a).await?;
        }
        Ok(())
    }
}
