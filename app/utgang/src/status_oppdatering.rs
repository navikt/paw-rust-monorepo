use std::{num::NonZeroU16, ops::Deref, sync::Arc};

use crate::pdl::pdl_query::PDLClient;
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

    pub fn kjoer_oppdatering() {}
}
