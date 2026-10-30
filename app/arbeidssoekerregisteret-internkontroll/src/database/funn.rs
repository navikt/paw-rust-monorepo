use sqlx::PgConnection;

use crate::regler::{alvorlighet::Utfall, nokkel::Nokkel, regel_katalog::RegelId};

pub async fn upsert_funn(
    conn: &mut PgConnection,
    regel: RegelId,
    regelversjon: i32,
    nokkel: &Nokkel,
    utfall: &Utfall,
    stream_ms: i64,
) -> Result<(), sqlx::Error> {
    match nokkel {
        Nokkel::Melding(xid) => {
            sqlx::query(
                r#"
                INSERT INTO funn_melding (
                    xid, regel, regelversjon, utfall, alvorlighet,
                    opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms
                )
                VALUES ($1, $2, $3, $4, $5, now(), now(), $6, $6)
                ON CONFLICT (xid, regel, regelversjon) DO UPDATE
                    SET utfall = EXCLUDED.utfall,
                        alvorlighet = EXCLUDED.alvorlighet,
                        sist_oppdatert = EXCLUDED.sist_oppdatert,
                        sist_oppdatert_stream_ms = EXCLUDED.sist_oppdatert_stream_ms
                    WHERE funn_melding.utfall IS DISTINCT FROM EXCLUDED.utfall
                "#,
            )
            .bind(xid)
            .bind(regel)
            .bind(regelversjon)
            .bind(utfall.kode)
            .bind(utfall.alvorlighet)
            .bind(stream_ms)
            .execute(&mut *conn)
            .await?;
        }
        Nokkel::Trace(trace_id) => {
            sqlx::query(
                r#"
                INSERT INTO funn_trace (
                    trace_id, regel, regelversjon, utfall, alvorlighet,
                    opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms
                )
                VALUES ($1, $2, $3, $4, $5, now(), now(), $6, $6)
                ON CONFLICT (trace_id, regel, regelversjon) DO UPDATE
                    SET utfall = EXCLUDED.utfall,
                        alvorlighet = EXCLUDED.alvorlighet,
                        sist_oppdatert = EXCLUDED.sist_oppdatert,
                        sist_oppdatert_stream_ms = EXCLUDED.sist_oppdatert_stream_ms
                    WHERE funn_trace.utfall IS DISTINCT FROM EXCLUDED.utfall
                "#,
            )
            .bind(trace_id)
            .bind(regel)
            .bind(regelversjon)
            .bind(utfall.kode)
            .bind(utfall.alvorlighet)
            .bind(stream_ms)
            .execute(&mut *conn)
            .await?;
        }
        Nokkel::Periode(periode_id) => {
            sqlx::query(
                r#"
                INSERT INTO funn_periode (
                    periode_id, regel, regelversjon, utfall, alvorlighet,
                    opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms
                )
                VALUES ($1, $2, $3, $4, $5, now(), now(), $6, $6)
                ON CONFLICT (periode_id, regel, regelversjon) DO UPDATE
                    SET utfall = EXCLUDED.utfall,
                        alvorlighet = EXCLUDED.alvorlighet,
                        sist_oppdatert = EXCLUDED.sist_oppdatert,
                        sist_oppdatert_stream_ms = EXCLUDED.sist_oppdatert_stream_ms
                    WHERE funn_periode.utfall IS DISTINCT FROM EXCLUDED.utfall
                "#,
            )
            .bind(periode_id)
            .bind(regel)
            .bind(regelversjon)
            .bind(utfall.kode)
            .bind(utfall.alvorlighet)
            .bind(stream_ms)
            .execute(&mut *conn)
            .await?;
        }
        Nokkel::Arbeidssoeker(arbeidssoeker_id) => {
            sqlx::query(
                r#"
                INSERT INTO funn_arbeidssoeker (
                    arbeidssoeker_id, regel, regelversjon, utfall, alvorlighet,
                    opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms
                )
                VALUES ($1, $2, $3, $4, $5, now(), now(), $6, $6)
                ON CONFLICT (arbeidssoeker_id, regel, regelversjon) DO UPDATE
                    SET utfall = EXCLUDED.utfall,
                        alvorlighet = EXCLUDED.alvorlighet,
                        sist_oppdatert = EXCLUDED.sist_oppdatert,
                        sist_oppdatert_stream_ms = EXCLUDED.sist_oppdatert_stream_ms
                    WHERE funn_arbeidssoeker.utfall IS DISTINCT FROM EXCLUDED.utfall
                "#,
            )
            .bind(arbeidssoeker_id)
            .bind(regel)
            .bind(regelversjon)
            .bind(utfall.kode)
            .bind(utfall.alvorlighet)
            .bind(stream_ms)
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

pub async fn delete_funn(
    conn: &mut PgConnection,
    regel: RegelId,
    regelversjon: i32,
    nokkel: &Nokkel,
) -> Result<(), sqlx::Error> {
    match nokkel {
        Nokkel::Melding(xid) => {
            sqlx::query(
                "DELETE FROM funn_melding WHERE xid = $1 AND regel = $2 AND regelversjon = $3",
            )
            .bind(xid)
            .bind(regel)
            .bind(regelversjon)
            .execute(&mut *conn)
            .await?;
        }
        Nokkel::Trace(trace_id) => {
            sqlx::query(
                "DELETE FROM funn_trace WHERE trace_id = $1 AND regel = $2 AND regelversjon = $3",
            )
            .bind(trace_id)
            .bind(regel)
            .bind(regelversjon)
            .execute(&mut *conn)
            .await?;
        }
        Nokkel::Periode(periode_id) => {
            sqlx::query(
                "DELETE FROM funn_periode WHERE periode_id = $1 AND regel = $2 AND regelversjon = $3",
            )
            .bind(periode_id)
            .bind(regel)
            .bind(regelversjon)
            .execute(&mut *conn)
            .await?;
        }
        Nokkel::Arbeidssoeker(arbeidssoeker_id) => {
            sqlx::query(
                "DELETE FROM funn_arbeidssoeker WHERE arbeidssoeker_id = $1 AND regel = $2 AND regelversjon = $3",
            )
            .bind(arbeidssoeker_id)
            .bind(regel)
            .bind(regelversjon)
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::xid8::Xid8;
    use crate::{database::trace::upsert_trace, regler::alvorlighet::Alvorlighet};
    use chrono::{DateTime, Utc};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use uuid::Uuid;

    type Stored = (i16, Alvorlighet, DateTime<Utc>, DateTime<Utc>, i64, i64);

    async fn stored(conn: &mut PgConnection, nokkel: &Nokkel, regelversjon: i32) -> Stored {
        match nokkel {
            Nokkel::Melding(xid) => {
                sqlx::query_as(
                    "SELECT utfall, alvorlighet, opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms FROM funn_melding WHERE xid = $1 AND regel = 2 AND regelversjon = $2",
                )
                .bind(xid)
                .bind(regelversjon)
                .fetch_one(&mut *conn)
                .await
                .unwrap()
            }
            Nokkel::Trace(trace_id) => {
                sqlx::query_as(
                    "SELECT utfall, alvorlighet, opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms FROM funn_trace WHERE trace_id = $1 AND regel = 2 AND regelversjon = $2",
                )
                .bind(trace_id)
                .bind(regelversjon)
                .fetch_one(&mut *conn)
                .await
                .unwrap()
            }
            Nokkel::Periode(periode_id) => {
                sqlx::query_as(
                    "SELECT utfall, alvorlighet, opprettet, sist_oppdatert, opprettet_stream_ms, sist_oppdatert_stream_ms FROM funn_periode WHERE periode_id = $1 AND regel = 2 AND regelversjon = $2",
                )
                .bind(periode_id)
                .bind(regelversjon)
                .fetch_one(&mut *conn)
                .await
                .unwrap()
            }
            Nokkel::Arbeidssoeker(_) => unreachable!("testen dekker ikke dette nivået"),
        }
    }

    async fn exists(conn: &mut PgConnection, nokkel: &Nokkel, regelversjon: i32) -> bool {
        match nokkel {
            Nokkel::Melding(xid) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM funn_melding WHERE xid = $1 AND regel = 2 AND regelversjon = $2)",
            )
            .bind(xid)
            .bind(regelversjon)
            .fetch_one(&mut *conn)
            .await
            .unwrap(),
            Nokkel::Trace(trace_id) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM funn_trace WHERE trace_id = $1 AND regel = 2 AND regelversjon = $2)",
            )
            .bind(trace_id)
            .bind(regelversjon)
            .fetch_one(&mut *conn)
            .await
            .unwrap(),
            Nokkel::Periode(periode_id) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM funn_periode WHERE periode_id = $1 AND regel = 2 AND regelversjon = $2)",
            )
            .bind(periode_id)
            .bind(regelversjon)
            .fetch_one(&mut *conn)
            .await
            .unwrap(),
            Nokkel::Arbeidssoeker(_) => unreachable!("testen dekker ikke dette nivået"),
        }
    }

    fn normal_utfall() -> Utfall {
        Utfall {
            kode: 3,
            alvorlighet: Alvorlighet::Advarsel,
        }
    }

    fn endret_utfall() -> Utfall {
        Utfall {
            kode: 4,
            alvorlighet: Alvorlighet::Kritisk,
        }
    }

    #[tokio::test]
    async fn oppdaterer_bare_ved_endret_utfall_paa_melding_trace_og_periode() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let trace_id = Uuid::from_u128(101);
        upsert_trace(&mut conn, trace_id, 1_000).await.unwrap();
        let nokler = [
            Nokkel::Melding(Xid8::new(101)),
            Nokkel::Trace(trace_id),
            Nokkel::Periode(Uuid::from_u128(201)),
        ];

        for nokkel in &nokler {
            upsert_funn(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                1,
                nokkel,
                &normal_utfall(),
                1_000,
            )
            .await
            .unwrap();
            let first = stored(&mut conn, nokkel, 1).await;
            assert_eq!(first.0, 3);
            assert_eq!(first.1, Alvorlighet::Advarsel);
            assert_eq!(first.4, 1_000);
            assert_eq!(first.5, 1_000);

            upsert_funn(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                1,
                nokkel,
                &Utfall {
                    kode: 3,
                    alvorlighet: Alvorlighet::Info,
                },
                2_000,
            )
            .await
            .unwrap();
            let unchanged = stored(&mut conn, nokkel, 1).await;
            assert_eq!(unchanged, first, "samme utfall skal ikke endre funnet");

            upsert_funn(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                1,
                nokkel,
                &endret_utfall(),
                3_000,
            )
            .await
            .unwrap();
            let updated = stored(&mut conn, nokkel, 1).await;
            assert_eq!(updated.0, 4);
            assert_eq!(updated.1, Alvorlighet::Kritisk);
            assert_eq!(updated.2, first.2, "opprettet skal beholdes");
            assert!(updated.3 > first.3, "sist_oppdatert skal endres");
            assert_eq!(updated.4, 1_000, "opprettet_stream_ms skal beholdes");
            assert_eq!(updated.5, 3_000);
        }
    }

    #[tokio::test]
    async fn sletting_er_selektiv_paa_melding_trace_og_periode() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let trace_id = Uuid::from_u128(301);
        upsert_trace(&mut conn, trace_id, 1_000).await.unwrap();
        let nokler = [
            Nokkel::Melding(Xid8::new(301)),
            Nokkel::Trace(trace_id),
            Nokkel::Periode(Uuid::from_u128(401)),
        ];

        for nokkel in &nokler {
            for regelversjon in [1, 2] {
                upsert_funn(
                    &mut conn,
                    RegelId::IngenPeriodeUtenStartet,
                    regelversjon,
                    nokkel,
                    &normal_utfall(),
                    1_000,
                )
                .await
                .unwrap();
            }
            delete_funn(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, nokkel)
                .await
                .unwrap();
            assert!(!exists(&mut conn, nokkel, 1).await);
            assert!(exists(&mut conn, nokkel, 2).await);
        }
    }

    #[tokio::test]
    async fn sorterer_alvorlighet_med_kritisk_foerst() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        for (xid, utfall) in [
            (501, normal_utfall()),
            (
                502,
                Utfall {
                    kode: 1,
                    alvorlighet: Alvorlighet::Info,
                },
            ),
            (503, endret_utfall()),
        ] {
            upsert_funn(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                1,
                &Nokkel::Melding(Xid8::new(xid)),
                &utfall,
                xid as i64,
            )
            .await
            .unwrap();
        }
        let sortert: Vec<Alvorlighet> =
            sqlx::query_scalar("SELECT alvorlighet FROM funn_melding ORDER BY alvorlighet DESC")
                .fetch_all(&mut *conn)
                .await
                .unwrap();
        assert_eq!(
            sortert,
            vec![
                Alvorlighet::Kritisk,
                Alvorlighet::Advarsel,
                Alvorlighet::Info
            ]
        );
    }
}
