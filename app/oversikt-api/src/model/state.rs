use sqlx::PgPool;

#[derive(Clone)]
pub struct RouterState {
    pub pg_pool: PgPool,
}

impl RouterState {
    pub const fn new(pg_pool: PgPool) -> Self {
        Self { pg_pool }
    }
}
