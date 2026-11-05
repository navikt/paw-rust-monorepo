use sqlx::PgConnection;

use crate::database::xid8::Xid8;
use crate::regler::{regel_katalog::RegelId, regelversjon::Regelversjon};

#[derive(Debug, PartialEq, Eq, Clone, sqlx::FromRow)]
pub struct RegelFremdrift {
    pub regel: RegelId,
    pub regelversjon: Regelversjon,
    pub siste_xid: Xid8,
}

#[tracing::instrument(skip(conn))]
pub async fn get_regel_fremdrift(
    conn: &mut PgConnection,
) -> Result<Vec<RegelFremdrift>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT regel, regelversjon, siste_xid
        FROM regel_fremdrift
        "#,
    )
    .fetch_all(&mut *conn)
    .await
}

#[tracing::instrument(skip(conn))]
pub async fn oppdater_regel_stream_ms(
    conn: &mut PgConnection,
    regel: RegelId,
    regelversjon: Regelversjon,
    stream_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE regel_fremdrift
        SET siste_stream_ms = GREATEST(siste_stream_ms, $3)
        WHERE regel = $1 AND regelversjon = $2
        "#,
    )
    .bind(regel)
    .bind(regelversjon)
    .bind(stream_ms)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Returns `true` when the row was created or moved forward, `false` when
/// `siste_xid` was not later than the stored value.
#[tracing::instrument(skip(conn))]
pub async fn upsert_regel_fremdrift(
    conn: &mut PgConnection,
    regel: RegelId,
    regelversjon: Regelversjon,
    siste_xid: Xid8,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO regel_fremdrift (regel, regelversjon, siste_xid)
        VALUES ($1, $2, $3)
        ON CONFLICT (regel, regelversjon) DO UPDATE
            SET siste_xid = EXCLUDED.siste_xid
            WHERE regel_fremdrift.siste_xid < EXCLUDED.siste_xid
        "#,
    )
    .bind(regel)
    .bind(regelversjon)
    .bind(siste_xid)
    .execute(&mut *conn)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    #[tokio::test]
    async fn tom_tabell_og_oppretting_og_okning_av_fremdrift() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        assert!(get_regel_fremdrift(&mut conn).await.unwrap().is_empty());

        upsert_regel_fremdrift(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(1),
            Xid8::new(100),
        )
        .await
        .unwrap();
        assert_eq!(
            get_regel_fremdrift(&mut conn).await.unwrap(),
            vec![RegelFremdrift {
                regel: RegelId::IngenPeriodeUtenStartet,
                regelversjon: Regelversjon::new(1),
                siste_xid: Xid8::new(100),
            }]
        );

        assert!(
            upsert_regel_fremdrift(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                Regelversjon::new(1),
                Xid8::new(200)
            )
            .await
            .unwrap()
        );
        assert_eq!(
            get_regel_fremdrift(&mut conn).await.unwrap(),
            vec![RegelFremdrift {
                regel: RegelId::IngenPeriodeUtenStartet,
                regelversjon: Regelversjon::new(1),
                siste_xid: Xid8::new(200),
            }]
        );
    }

    #[tokio::test]
    async fn lik_eller_lavere_xid_endrer_ikke_fremdrift() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        assert!(
            upsert_regel_fremdrift(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                Regelversjon::new(1),
                Xid8::new(100)
            )
            .await
            .unwrap()
        );
        assert!(
            !upsert_regel_fremdrift(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                Regelversjon::new(1),
                Xid8::new(100)
            )
            .await
            .unwrap()
        );
        assert!(
            !upsert_regel_fremdrift(
                &mut conn,
                RegelId::IngenPeriodeUtenStartet,
                Regelversjon::new(1),
                Xid8::new(99)
            )
            .await
            .unwrap()
        );

        assert_eq!(
            get_regel_fremdrift(&mut conn).await.unwrap(),
            vec![RegelFremdrift {
                regel: RegelId::IngenPeriodeUtenStartet,
                regelversjon: Regelversjon::new(1),
                siste_xid: Xid8::new(100),
            }]
        );
    }

    #[tokio::test]
    async fn regler_og_versjoner_er_uavhengige_og_transaksjon_kan_rulles_tilbake() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        upsert_regel_fremdrift(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(1),
            Xid8::new(100),
        )
        .await
        .unwrap();
        upsert_regel_fremdrift(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(2),
            Xid8::new(200),
        )
        .await
        .unwrap();
        upsert_regel_fremdrift(
            &mut conn,
            RegelId::HeleTraceSignert,
            Regelversjon::new(1),
            Xid8::new(300),
        )
        .await
        .unwrap();

        let rows = get_regel_fremdrift(&mut conn).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.contains(&RegelFremdrift {
            regel: RegelId::IngenPeriodeUtenStartet,
            regelversjon: Regelversjon::new(1),
            siste_xid: Xid8::new(100),
        }));
        assert!(rows.contains(&RegelFremdrift {
            regel: RegelId::IngenPeriodeUtenStartet,
            regelversjon: Regelversjon::new(2),
            siste_xid: Xid8::new(200),
        }));
        assert!(rows.contains(&RegelFremdrift {
            regel: RegelId::HeleTraceSignert,
            regelversjon: Regelversjon::new(1),
            siste_xid: Xid8::new(300),
        }));
    }

    #[tokio::test]
    async fn regel_stream_tid_settes_og_gaar_ikke_bakover() {
        async fn hent_stream_ms(conn: &mut PgConnection) -> Option<i64> {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT siste_stream_ms FROM regel_fremdrift WHERE regel = $1 AND regelversjon = $2",
            )
            .bind(RegelId::IngenPeriodeUtenStartet)
            .bind(Regelversjon::new(1))
            .fetch_one(conn)
            .await
            .unwrap()
        }

        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        upsert_regel_fremdrift(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(1),
            Xid8::START,
        )
        .await
        .unwrap();
        assert_eq!(hent_stream_ms(&mut conn).await, None);
        oppdater_regel_stream_ms(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(1),
            100,
        )
        .await
        .unwrap();
        assert_eq!(hent_stream_ms(&mut conn).await, Some(100));
        oppdater_regel_stream_ms(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(1),
            50,
        )
        .await
        .unwrap();
        assert_eq!(hent_stream_ms(&mut conn).await, Some(100));
        oppdater_regel_stream_ms(
            &mut conn,
            RegelId::IngenPeriodeUtenStartet,
            Regelversjon::new(1),
            200,
        )
        .await
        .unwrap();
        assert_eq!(hent_stream_ms(&mut conn).await, Some(200));
    }
}
