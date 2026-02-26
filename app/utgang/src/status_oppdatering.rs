use crate::pdl::pdl_query::PDLClient;
use sqlx::PgPool;

pub struct StatusOppdatering {
    pg_pool: PgPool,
    pdl_client: PDLClient,
}
