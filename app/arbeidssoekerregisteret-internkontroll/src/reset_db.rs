use sqlx::Acquire;
use sqlx::PgPool;
use tracing::info;

const RESET_SQL: &str = include_str!("../reset.sql");
pub async fn reset_db_if_needed(
    pool: &PgPool,
    configured_hwm_version: i16,
) -> Result<bool, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('hwm') IS NOT NULL")
        .fetch_one(&mut *connection)
        .await?;
    if !hwm_exists {
        info!("HWM-tabellen mangler, hopper over nullstilling av DB");
        return Ok(false);
    }
    let max_hwm_version: Option<i16> = sqlx::query_scalar("SELECT MAX(version) FROM hwm")
        .fetch_one(&mut *connection)
        .await?;
    match max_hwm_version {
        Some(version) if configured_hwm_version <= version => {
            info!(
                configured_hwm_version,
                stored_hwm_version = version,
                "Konfigurert HWM-versjon er ikke nyere enn lagret versjon, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(version) => {
            info!(
                configured_hwm_version,
                stored_hwm_version = version,
                "Konfigurert HWM-versjon er nyere enn lagret versjon, nullstiller DB"
            );
        }
        None => {
            info!(
                configured_hwm_version,
                "HWM-tabellen er tom, nullstiller DB"
            );
        }
    }
    let mut transaction = connection.begin().await?;
    sqlx::raw_sql(RESET_SQL).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    #[tokio::test]
    async fn nullstilling_fjerner_alt_ogsaa_tabeller_fra_eldre_migrasjoner() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        sqlx::raw_sql(
            "CREATE TABLE kafka_record_data (id BIGINT PRIMARY KEY);
             CREATE TABLE kafka_header (data_id BIGINT NOT NULL REFERENCES kafka_record_data (id));
             ALTER TABLE kafka_record ADD COLUMN data_id BIGINT REFERENCES kafka_record_data (id);
             CREATE TABLE signature_boundary (
                 topic_id SMALLINT, record_partition INTEGER, record_offset BIGINT,
                 FOREIGN KEY (topic_id, record_partition, record_offset)
                     REFERENCES kafka_record (topic_id, record_partition, record_offset));
             CREATE TABLE signing_key (id TEXT PRIMARY KEY);
             INSERT INTO hwm (version, topic, partition, hwm) VALUES (1, 'topic', 0, 0);",
        )
        .execute(pool)
        .await
        .unwrap();

        assert!(reset_db_if_needed(pool, i16::MAX).await.unwrap());

        let remaining: Vec<String> = sqlx::query_scalar(
            "SELECT tablename::text FROM pg_tables WHERE schemaname = current_schema()
             UNION ALL
             SELECT typname::text FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace
             WHERE n.nspname = current_schema() AND t.typtype = 'e'",
        )
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(remaining, Vec::<String>::new());
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
    }
}
