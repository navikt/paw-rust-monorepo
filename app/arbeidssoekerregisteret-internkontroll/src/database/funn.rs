use sqlx::PgExecutor;
use uuid::Uuid;

pub const UTFALL_OK: i16 = 0;
pub const UTFALL_TEKNISK_FEIL: i16 = -1;

pub async fn upsert_funn(
    executor: impl PgExecutor<'_>,
    trace_id: Uuid,
    regel: i16,
    regelversjon: i32,
    utfall: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO funn (trace_id, regel, regelversjon, utfall, opprettet, sist_oppdatert)
        VALUES ($1, $2, $3, $4, now(), now())
        ON CONFLICT (trace_id, regel, regelversjon) DO UPDATE
            SET utfall = EXCLUDED.utfall,
                sist_oppdatert = EXCLUDED.sist_oppdatert
            WHERE funn.utfall IS DISTINCT FROM EXCLUDED.utfall
        "#,
    )
    .bind(trace_id)
    .bind(regel)
    .bind(regelversjon)
    .bind(utfall)
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::trace::upsert_trace;
    use chrono::{DateTime, Utc};
    use postgres_testcontainer::postgres::setup_postgres_container;

    async fn stored(pool: &sqlx::PgPool, trace_id: Uuid) -> (i16, DateTime<Utc>, DateTime<Utc>) {
        sqlx::query_as("SELECT utfall, opprettet, sist_oppdatert FROM funn WHERE trace_id = $1")
            .bind(trace_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn sist_oppdatert_endres_bare_naar_utfallet_endres() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let trace_id = Uuid::from_u128(1);
        upsert_trace(pool, trace_id, 1_000, 1).await.unwrap();

        upsert_funn(pool, trace_id, 1, 1, 3).await.unwrap();
        let (utfall, opprettet, sist_oppdatert) = stored(pool, trace_id).await;
        assert_eq!(utfall, 3);
        assert_eq!(sist_oppdatert, opprettet);

        upsert_funn(pool, trace_id, 1, 1, 3).await.unwrap();
        assert_eq!(stored(pool, trace_id).await, (3, opprettet, opprettet));

        upsert_funn(pool, trace_id, 1, 1, UTFALL_OK).await.unwrap();
        let (utfall, opprettet_etter, sist_oppdatert) = stored(pool, trace_id).await;
        assert_eq!(utfall, UTFALL_OK);
        assert_eq!(opprettet_etter, opprettet);
        assert!(sist_oppdatert > opprettet);
    }
}
