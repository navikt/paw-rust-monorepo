use sqlx::PgExecutor;

use crate::topic_ids::TopicIds;

/// Registers the physical topic names in the `topic` table.
/// Topics that are already registered keep their existing ID.
pub async fn register_topics(
    executor: impl PgExecutor<'_>,
    topics: &[&str],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO topic (name)
        SELECT UNNEST($1::VARCHAR[])
        ON CONFLICT (name) DO NOTHING
        "#,
    )
    .bind(topics)
    .execute(executor)
    .await?;
    Ok(())
}

/// Reads all registered topics into a [`TopicIds`] lookup.
pub async fn load_topic_ids(executor: impl PgExecutor<'_>) -> Result<TopicIds, sqlx::Error> {
    let rows: Vec<(String, i16)> = sqlx::query_as("SELECT name, id FROM topic")
        .fetch_all(executor)
        .await?;
    Ok(TopicIds::new(rows))
}
