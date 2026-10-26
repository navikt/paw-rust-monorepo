use sqlx::PgConnection;

use crate::topic_ids::TopicIds;

/// Registers the physical topic names in the `topic` table.
/// Topics that are already registered keep their existing ID.
pub async fn register_topics(conn: &mut PgConnection, topics: &[&str]) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO topic (name)
        SELECT UNNEST($1::VARCHAR[])
        ON CONFLICT (name) DO NOTHING
        "#,
    )
    .bind(topics)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Reads all registered topics into a [`TopicIds`] lookup.
pub async fn load_topic_ids(conn: &mut PgConnection) -> Result<TopicIds, sqlx::Error> {
    let rows: Vec<(String, i16)> = sqlx::query_as("SELECT name, id FROM topic")
        .fetch_all(&mut *conn)
        .await?;
    Ok(TopicIds::new(rows))
}
