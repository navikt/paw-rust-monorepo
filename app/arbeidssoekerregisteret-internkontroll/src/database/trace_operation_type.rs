use sqlx::PgExecutor;
use uuid::Uuid;

use crate::trace_operation_type::{TraceOperationType, UnknownTraceOperationType};

#[derive(Debug, thiserror::Error)]
pub enum TraceOperationTypeError {
    #[error("trace already has operation type {0:?}")]
    AlreadyExists(TraceOperationType),
    #[error(transparent)]
    UnknownStoredType(#[from] UnknownTraceOperationType),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

/// Stores the operation type of a trace. The type is written once.
///
/// Storing the same type again succeeds, so a classification that is retried
/// after a crash does not fail. A different stored type gives `AlreadyExists`.
pub async fn insert_trace_operation_type(
    executor: impl PgExecutor<'_>,
    trace_id: Uuid,
    operation_type: TraceOperationType,
) -> Result<(), TraceOperationTypeError> {
    let stored: i16 = sqlx::query_scalar(
        r#"
        WITH inserted AS (
            INSERT INTO trace_operation_type (trace_id, operation_type)
            VALUES ($1, $2)
            ON CONFLICT (trace_id) DO NOTHING
            RETURNING operation_type
        )
        SELECT operation_type FROM inserted
        UNION ALL
        SELECT operation_type FROM trace_operation_type WHERE trace_id = $1
        LIMIT 1
        "#,
    )
    .bind(trace_id)
    .bind(i16::from(operation_type))
    .fetch_one(executor)
    .await?;

    let stored = TraceOperationType::try_from(stored)?;
    if stored == operation_type {
        Ok(())
    } else {
        Err(TraceOperationTypeError::AlreadyExists(stored))
    }
}
