use sqlx::PgConnection;

/// Returns the stored stream time for a topic-partition, or `None` if none is stored.
#[tracing::instrument(skip(conn))]
pub async fn get_stream_time(
    conn: &mut PgConnection,
    topic_id: i16,
    topic_partition: i32,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT timestamp_ms
        FROM stream_time
        WHERE topic_id = $1 AND topic_partition = $2
        "#,
    )
    .bind(topic_id)
    .bind(topic_partition)
    .fetch_optional(&mut *conn)
    .await
}

/// Moves the stream time for a topic-partition forward to `timestamp_ms` when it
/// is later than the stored value, and creates the row if it is missing.
///
/// Returns `timestamp_ms` minus the previously stored timestamp, whether or not
/// the row was updated: positive means later, negative means earlier.
/// Returns `None` when the topic-partition had no stored timestamp.
#[tracing::instrument(skip(conn))]
pub async fn upsert_stream_time(
    conn: &mut PgConnection,
    topic_id: i16,
    topic_partition: i32,
    timestamp_ms: i64,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        WITH previous AS (
            SELECT timestamp_ms
            FROM stream_time
            WHERE topic_id = $1 AND topic_partition = $2
            FOR UPDATE
        ),
        upsert AS (
            INSERT INTO stream_time (topic_id, topic_partition, timestamp_ms)
            VALUES ($1, $2, $3)
            ON CONFLICT (topic_id, topic_partition) DO UPDATE
                SET timestamp_ms = EXCLUDED.timestamp_ms
                WHERE stream_time.timestamp_ms < EXCLUDED.timestamp_ms
        )
        SELECT $3 - (SELECT timestamp_ms FROM previous)
        "#,
    )
    .bind(topic_id)
    .bind(topic_partition)
    .bind(timestamp_ms)
    .fetch_one(&mut *conn)
    .await
}
