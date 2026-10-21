use sqlx::Acquire;
use sqlx::PgPool;
use tracing::info;

const RESET_SQL: &str = include_str!("../reset.sql");
#[derive(Debug, thiserror::Error)]
pub enum ResetDbError {
    #[error("stored HWM version {stored} is newer than configured version {configured}")]
    StoredVersionNewer { stored: i16, configured: i16 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

pub async fn reset_db_if_needed(
    pool: &PgPool,
    configured_hwm_version: i16,
) -> Result<bool, ResetDbError> {
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
        None => {
            info!(
                configured_hwm_version,
                "HWM-tabellen er tom, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(stored) if stored == configured_hwm_version => {
            info!(
                configured_hwm_version,
                "HWM-versjonen er uendret, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(stored) if stored > configured_hwm_version => {
            return Err(ResetDbError::StoredVersionNewer {
                stored,
                configured: configured_hwm_version,
            });
        }
        Some(stored) => {
            info!(
                configured_hwm_version,
                stored_hwm_version = stored,
                "HWM-versjonen er økt, nullstiller DB"
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

    async fn database_with_hwm_version(
        version: Option<i16>,
    ) -> postgres_testcontainer::postgres::TestContainerGuard {
        let guard = setup_postgres_container().await.unwrap();
        sqlx::migrate!("./migrations")
            .run(&guard.pg_pool)
            .await
            .unwrap();
        if let Some(version) = version {
            sqlx::query(
                "INSERT INTO hwm (version, topic, partition, hwm) VALUES ($1, 'topic', 0, 0)",
            )
            .bind(version)
            .execute(&guard.pg_pool)
            .await
            .unwrap();
        }
        guard
    }

    async fn hwm_rows(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM hwm")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn uendret_versjon_nullstiller_ikke() {
        let guard = database_with_hwm_version(Some(5)).await;
        assert!(!reset_db_if_needed(&guard.pg_pool, 5).await.unwrap());
        assert_eq!(hwm_rows(&guard.pg_pool).await, 1);
    }

    #[tokio::test]
    async fn tom_hwm_nullstiller_ikke() {
        let guard = database_with_hwm_version(None).await;
        sqlx::query("INSERT INTO topic (name) VALUES ('topic')")
            .execute(&guard.pg_pool)
            .await
            .unwrap();
        assert!(!reset_db_if_needed(&guard.pg_pool, 5).await.unwrap());
        let topics: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topic")
            .fetch_one(&guard.pg_pool)
            .await
            .unwrap();
        assert_eq!(topics, 1);
    }

    #[tokio::test]
    async fn eldre_konfigurert_versjon_feiler_uten_aa_nullstille() {
        let guard = database_with_hwm_version(Some(5)).await;
        assert!(matches!(
            reset_db_if_needed(&guard.pg_pool, 4).await,
            Err(ResetDbError::StoredVersionNewer {
                stored: 5,
                configured: 4
            })
        ));
        assert_eq!(hwm_rows(&guard.pg_pool).await, 1);
    }

    #[tokio::test]
    async fn oekt_versjon_nullstiller() {
        let guard = database_with_hwm_version(Some(5)).await;
        assert!(reset_db_if_needed(&guard.pg_pool, 6).await.unwrap());
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT to_regclass('hwm') IS NULL")
                .fetch_one(&guard.pg_pool)
                .await
                .unwrap()
        );
    }
}
