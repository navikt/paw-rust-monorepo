use sqlx::PgPool;

#[derive(Clone)]
pub struct AppContext {
    pub db: PgPool,
}
