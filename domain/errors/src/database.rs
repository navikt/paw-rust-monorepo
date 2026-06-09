use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DatabaseError {
    #[error("Failed to initialize database connection pool: {0}")]
    InitializePool(#[from] sqlx::Error),
    #[error("Failed to verify database connection: {0}")]
    VerifyConnection(sqlx::Error),
    #[error("Failed to migrate schema changes: {0}")]
    MigrateSchema(sqlx::migrate::MigrateError),
    #[error("Failed to execute query: {0}")]
    ExecuteQuery(sqlx::Error),
}
