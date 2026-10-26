use sqlx::PgConnection;
use uuid::Uuid;

use crate::regler::regel_katalog::RegelId;

pub async fn upsert_funn(
    conn: &mut PgConnection,
    trace_id: Uuid,
    regel: RegelId,
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
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn delete_funn(
    conn: &mut PgConnection,
    trace_id: Uuid,
    regel: RegelId,
    regelversjon: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM funn
        WHERE trace_id = $1 AND regel = $2 AND regelversjon = $3
        "#,
    )
    .bind(trace_id)
    .bind(regel)
    .bind(regelversjon)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::trace::upsert_trace;
    use chrono::{DateTime, Utc};
    use postgres_testcontainer::postgres::setup_postgres_container;

    async fn stored(
        conn: &mut sqlx::PgConnection,
        trace_id: Uuid,
    ) -> (i16, DateTime<Utc>, DateTime<Utc>) {
        sqlx::query_as("SELECT utfall, opprettet, sist_oppdatert FROM funn WHERE trace_id = $1")
            .bind(trace_id)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn sist_oppdatert_endres_bare_naar_utfallet_endres() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let trace_id = Uuid::from_u128(1);
        upsert_trace(&mut conn, trace_id, 1_000).await.unwrap();

        upsert_funn(&mut conn, trace_id, RegelId::IngenPeriodeUtenStartet, 1, 3)
            .await
            .unwrap();
        let (utfall, opprettet, sist_oppdatert) = stored(&mut conn, trace_id).await;
        assert_eq!(utfall, 3);
        assert_eq!(sist_oppdatert, opprettet);

        upsert_funn(&mut conn, trace_id, RegelId::IngenPeriodeUtenStartet, 1, 3)
            .await
            .unwrap();
        assert_eq!(stored(&mut conn, trace_id).await, (3, opprettet, opprettet));

        upsert_funn(&mut conn, trace_id, RegelId::IngenPeriodeUtenStartet, 1, 0)
            .await
            .unwrap();
        let (utfall, opprettet_etter, sist_oppdatert) = stored(&mut conn, trace_id).await;
        assert_eq!(utfall, 0);
        assert_eq!(opprettet_etter, opprettet);
        assert!(sist_oppdatert > opprettet);
    }

    #[tokio::test]
    async fn sletting_av_funn() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let trace_id = Uuid::from_u128(1);
        let trace_id2 = Uuid::from_u128(2);
        upsert_trace(&mut conn, trace_id, 1_000).await.unwrap();
        upsert_trace(&mut conn, trace_id2, 1_000).await.unwrap();

        upsert_funn(&mut conn, trace_id, RegelId::IngenPeriodeUtenStartet, 1, 3)
            .await
            .unwrap();
        upsert_funn(&mut conn, trace_id2, RegelId::IngenPeriodeUtenStartet, 1, 3)
            .await
            .unwrap();
        assert_eq!(stored(&mut conn, trace_id).await.0, 3);

        delete_funn(&mut conn, trace_id, RegelId::IngenPeriodeUtenStartet, 1)
            .await
            .unwrap();
        let result = sqlx::query("SELECT utfall FROM funn WHERE trace_id = $1")
            .bind(trace_id)
            .fetch_optional(&mut *conn)
            .await
            .unwrap();
        assert!(result.is_none());
        let result2 = sqlx::query("SELECT utfall FROM funn WHERE trace_id = $1")
            .bind(trace_id2)
            .fetch_optional(&mut *conn)
            .await
            .unwrap();
        assert!(result2.is_some());
    }
}
