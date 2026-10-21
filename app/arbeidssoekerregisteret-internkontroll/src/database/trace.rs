use sqlx::PgExecutor;
use uuid::Uuid;

use crate::trace_operation_type::{TraceOperationType, UnknownTraceOperationType};

pub async fn upsert_trace(
    executor: impl PgExecutor<'_>,
    trace_id: Uuid,
    reference_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trace (trace_id, first_reference_ms, last_reference_ms)
        VALUES ($1, $2, $2)
        ON CONFLICT (trace_id) DO UPDATE
            SET first_reference_ms = LEAST(trace.first_reference_ms, EXCLUDED.first_reference_ms),
                last_reference_ms = GREATEST(trace.last_reference_ms, EXCLUDED.last_reference_ms)
        "#,
    )
    .bind(trace_id)
    .bind(reference_ms)
    .execute(executor)
    .await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum TraceOperationTypeError {
    #[error("trace not found")]
    TraceNotFound,
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
pub async fn set_trace_operation_type(
    executor: impl PgExecutor<'_>,
    trace_id: Uuid,
    operation_type: TraceOperationType,
) -> Result<(), TraceOperationTypeError> {
    let stored: Option<i16> = sqlx::query_scalar(
        r#"
        UPDATE trace
        SET operation_type = COALESCE(operation_type, $2)
        WHERE trace_id = $1
        RETURNING operation_type
        "#,
    )
    .bind(trace_id)
    .bind(i16::from(operation_type))
    .fetch_optional(executor)
    .await?;

    let stored =
        TraceOperationType::try_from(stored.ok_or(TraceOperationTypeError::TraceNotFound)?)?;
    if stored == operation_type {
        Ok(())
    } else {
        Err(TraceOperationTypeError::AlreadyExists(stored))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    async fn reference_times(pool: &sqlx::PgPool, trace_id: Uuid) -> (i64, i64) {
        sqlx::query_as(
            "SELECT first_reference_ms, last_reference_ms FROM trace WHERE trace_id = $1",
        )
        .bind(trace_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn foerste_referanse_gaar_bare_bakover_og_siste_bare_fremover() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let trace_id = Uuid::from_u128(1);

        upsert_trace(pool, trace_id, 1_000).await.unwrap();
        assert_eq!(reference_times(pool, trace_id).await, (1_000, 1_000));
        upsert_trace(pool, trace_id, 3_000).await.unwrap();
        assert_eq!(reference_times(pool, trace_id).await, (1_000, 3_000));
        upsert_trace(pool, trace_id, 500).await.unwrap();
        assert_eq!(reference_times(pool, trace_id).await, (500, 3_000));
        upsert_trace(pool, trace_id, 2_000).await.unwrap();
        assert_eq!(reference_times(pool, trace_id).await, (500, 3_000));

        let sequence = |id: Uuid| {
            sqlx::query_scalar::<_, i64>("SELECT sequence_number FROM trace WHERE trace_id = $1")
                .bind(id)
                .fetch_one(pool)
        };
        let first_sequence = sequence(trace_id).await.unwrap();
        upsert_trace(pool, Uuid::from_u128(2), 4_000).await.unwrap();
        assert!(sequence(Uuid::from_u128(2)).await.unwrap() > first_sequence);
        assert_eq!(sequence(trace_id).await.unwrap(), first_sequence);
    }

    #[tokio::test]
    async fn operasjonstype_skrives_en_gang() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let trace_id = Uuid::from_u128(1);

        assert!(matches!(
            set_trace_operation_type(pool, trace_id, TraceOperationType::PeriodeStartet).await,
            Err(TraceOperationTypeError::TraceNotFound)
        ));
        upsert_trace(pool, trace_id, 1_000).await.unwrap();
        set_trace_operation_type(pool, trace_id, TraceOperationType::PeriodeStartet)
            .await
            .unwrap();
        set_trace_operation_type(pool, trace_id, TraceOperationType::PeriodeStartet)
            .await
            .unwrap();
        assert!(matches!(
            set_trace_operation_type(pool, trace_id, TraceOperationType::PeriodeStoppet).await,
            Err(TraceOperationTypeError::AlreadyExists(
                TraceOperationType::PeriodeStartet
            ))
        ));
    }
}
