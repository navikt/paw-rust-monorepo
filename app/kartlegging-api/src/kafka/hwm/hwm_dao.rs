use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR")]
pub enum HwmStatus {
    #[sqlx(rename = "ACTIVE")]
    Active,
    #[sqlx(rename = "PAUSED")]
    Paused,
}

#[derive(Debug, FromRow)]
pub struct HwmWithStatusRow {
    pub version: i16,
    pub topic: String,
    pub partition: i32,
    pub hwm: i64,
    pub status: HwmStatus,
    pub status_timestamp: DateTime<Utc>,
}

pub async fn get_by_topic_partition(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
) -> anyhow::Result<Option<HwmWithStatusRow>> {
    let row = sqlx::query_as::<_, HwmWithStatusRow>(
        r#"
        SELECT
            version,
            topic,
            partition,
            hwm,
            status,
            status_timestamp AT TIME ZONE 'UTC' AS status_timestamp
        FROM hwm
        WHERE version = $1 AND topic = $2 AND partition = $3;
        "#,
    )
    .bind(version)
    .bind(topic)
    .bind(partition)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

pub async fn find_by_status(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    status: HwmStatus,
) -> anyhow::Result<Vec<HwmWithStatusRow>> {
    let rows = sqlx::query_as::<_, HwmWithStatusRow>(
        r#"
        SELECT
            version,
            topic,
            partition,
            hwm,
            status,
            status_timestamp AT TIME ZONE 'UTC' AS status_timestamp
        FROM hwm
        WHERE version = $1 AND status = $2;
        "#,
    )
    .bind(version)
    .bind(status)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows)
}

pub async fn update_as_paused(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE hwm
        SET status = 'PAUSED', status_timestamp = CURRENT_TIMESTAMP(6)
        WHERE version = $1 AND topic = $2 AND partition = $3 AND status != 'PAUSED';
        "#,
    )
    .bind(version)
    .bind(topic)
    .bind(partition)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn update_as_active(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE hwm
        SET status = 'ACTIVE', status_timestamp = CURRENT_TIMESTAMP(6)
        WHERE version = $1 AND topic = $2 AND partition = $3 AND status = 'PAUSED';
        "#,
    )
    .bind(version)
    .bind(topic)
    .bind(partition)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use std::collections::BTreeMap;
    use tokio::sync::OnceCell;

    #[test]
    fn statuser_er_sammenlignbare() {
        assert_eq!(HwmStatus::Active, HwmStatus::Active);
        assert_ne!(HwmStatus::Active, HwmStatus::Paused);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ny_rad_er_active_som_default() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        context.insert_hwm_row(&mut tx, 1, "topic-1", 0).await;

        let row = get_by_topic_partition(&mut tx, 1, "topic-1", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(matches!(
            row,
            HwmWithStatusRow {
                status: HwmStatus::Active,
                ..
            }
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mark_paused_setter_status_og_tidsstempel() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        context.insert_hwm_row(&mut tx, 1, "topic-2", 0).await;

        update_as_paused(&mut tx, 1, "topic-2", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        let row = get_by_topic_partition(&mut tx, 1, "topic-2", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(matches!(
            row,
            HwmWithStatusRow {
                status: HwmStatus::Paused,
                ..
            }
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mark_paused_er_guardet_mot_a_friske_opp_tidsstempel() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        context.insert_hwm_row(&mut tx, 1, "topic-3", 0).await;

        update_as_paused(&mut tx, 1, "topic-3", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        let row_1 = get_by_topic_partition(&mut tx, 1, "topic-3", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        update_as_paused(&mut tx, 1, "topic-3", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        let row_2 = get_by_topic_partition(&mut tx, 1, "topic-3", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(row_1.status, HwmStatus::Paused);
        assert_eq!(row_2.status, HwmStatus::Paused);
        assert_eq!(row_1.status_timestamp, row_2.status_timestamp);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mark_resolved_setter_status_tilbake_til_active() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        context.insert_hwm_row(&mut tx, 1, "topic-4", 0).await;

        update_as_paused(&mut tx, 1, "topic-4", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        update_as_active(&mut tx, 1, "topic-4", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        let row = get_by_topic_partition(&mut tx, 1, "topic-4", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(matches!(
            row,
            HwmWithStatusRow {
                status: HwmStatus::Active,
                ..
            }
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mark_resolved_er_guardet_mot_rader_som_ikke_er_paused() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        context.insert_hwm_row(&mut tx, 1, "topic-5", 0).await;

        let row_1 = get_by_topic_partition(&mut tx, 1, "topic-5", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        update_as_active(&mut tx, 1, "topic-5", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        let row_2 = get_by_topic_partition(&mut tx, 1, "topic-5", 0)
            .await
            .expect("Kunne ikke hente hwm")
            .expect("Ingen hwm funnet");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(row_1.status, HwmStatus::Active);
        assert_eq!(row_2.status, HwmStatus::Active);
        assert_eq!(row_1.status_timestamp, row_2.status_timestamp);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_paused_returnerer_kun_pausede_rader_for_riktig_versjon() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        context.insert_hwm_row(&mut tx, 1, "topic-6", 0).await;
        context.insert_hwm_row(&mut tx, 1, "topic-7", 1).await;
        context.insert_hwm_row(&mut tx, 2, "topic-8", 0).await;

        update_as_paused(&mut tx, 1, "topic-6", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");
        update_as_paused(&mut tx, 2, "topic-6", 0)
            .await
            .expect("Kunne ikke oppdatere hwm");

        let rows = find_by_status(&mut tx, 1, HwmStatus::Paused)
            .await
            .expect("Kunne ikke hente hwm");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        let row_map: BTreeMap<String, i32> = rows
            .iter()
            .map(|row| (row.topic.clone(), row.partition))
            .collect();
        let topic_6_partition = row_map.get("topic-6").expect("topic-6 ikke funnet");
        assert_eq!(*topic_6_partition, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_status_returnerer_none_for_ukjent_rad() {
        let context = init().await;
        let mut tx = context.start_tx().await;

        let row = get_by_topic_partition(&mut tx, 1, "ukjent-topic", 0)
            .await
            .expect("Kunne ikke hente hwm");

        assert!(row.is_none());
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let postgres_guard = setup_postgres_container()
                .await
                .expect("Failed to start Postgres container");
            println!("Migrerer databasemodell");
            sqlx::migrate!("./migrations")
                .run(&postgres_guard.pg_pool)
                .await
                .expect("Failed to run migrations");

            TestContext {
                pg_pool: postgres_guard.pg_pool,
            }
        })
        .await
    }

    struct TestContext {
        pg_pool: PgPool,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }

        async fn insert_hwm_row(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            version: i16,
            topic: &str,
            partition: i32,
        ) {
            sqlx::query(
                r#"
            INSERT INTO hwm (version, topic, partition, hwm)
            VALUES ($1, $2, $3, 0);
            "#,
            )
            .bind(version)
            .bind(topic)
            .bind(partition)
            .execute(&mut **tx)
            .await
            .expect("Kunne ikke sette inn hwm-rad for testoppsett");
        }
    }
}
