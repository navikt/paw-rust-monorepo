use arbeidssoekerregisteret_internkontroll::reset_db::{ResetDbError, reset_db_if_needed};
use postgres_testcontainer::postgres::setup_postgres_container;
use sqlx::PgPool;

async fn sett_hwm_versjon(pool: &PgPool, versjon: i16) {
    sqlx::query("INSERT INTO hwm (version, topic, partition, hwm) VALUES ($1, 'topic', 0, 0)")
        .bind(versjon)
        .execute(pool)
        .await
        .unwrap();
}

async fn er_tom(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT (SELECT COUNT(*) FROM pg_tables WHERE schemaname = current_schema())
              + (SELECT COUNT(*) FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace
                 WHERE n.nspname = current_schema() AND t.typtype = 'e')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
        == 0
}

#[tokio::test]
async fn nullstilling_styres_av_hwm_versjon() {
    let pg = setup_postgres_container().await.unwrap();
    let pool = &pg.pg_pool;
    sqlx::migrate!("./migrations").run(pool).await.unwrap();

    assert!(
        !reset_db_if_needed(pool, 5).await.unwrap(),
        "tom hwm nullstiller ikke"
    );
    sett_hwm_versjon(pool, 5).await;
    assert!(
        !reset_db_if_needed(pool, 5).await.unwrap(),
        "samme versjon nullstiller ikke"
    );
    assert!(matches!(
        reset_db_if_needed(pool, 4).await,
        Err(ResetDbError::StoredVersionNewer {
            stored: 5,
            configured: 4
        })
    ));
    assert!(reset_db_if_needed(pool, 6).await.unwrap());
    assert!(er_tom(pool).await);
}

/// Migreringene fra 07fa59c, siste versjon før omskrivingen til record_metadata.
#[tokio::test]
async fn nullstilling_fjerner_gammel_struktur() {
    let pg = setup_postgres_container().await.unwrap();
    let pool = &pg.pg_pool;
    sqlx::migrate!("./test_fixtures/reset_db/gammel_struktur")
        .run(pool)
        .await
        .unwrap();
    sett_hwm_versjon(pool, 1).await;
    assert!(reset_db_if_needed(pool, 2).await.unwrap());
    assert!(er_tom(pool).await);
    sqlx::migrate!("./migrations").run(pool).await.unwrap();
}
