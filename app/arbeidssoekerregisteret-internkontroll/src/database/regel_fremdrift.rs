use sqlx::PgConnection;

use crate::regler::regel_katalog::RegelId;

#[derive(Debug, PartialEq, Eq, Clone, sqlx::FromRow)]
pub struct RegelFremdrift {
    pub regel: RegelId,
    pub regelversjon: i32,
    pub siste_xid: i64,
}

#[tracing::instrument(skip(conn))]
pub async fn get_regel_fremdrift(
    conn: &mut PgConnection,
) -> Result<Vec<RegelFremdrift>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT regel, regelversjon, siste_xid::text::bigint AS siste_xid
        FROM regel_fremdrift
        "#,
    )
    .fetch_all(&mut *conn)
    .await
}

/// Returns `true` when the row was created or moved forward, `false` when
/// `siste_xid` was not later than the stored value.
#[tracing::instrument(skip(conn))]
pub async fn upsert_regel_fremdrift(
    conn: &mut PgConnection,
    regel: RegelId,
    regelversjon: i32,
    siste_xid: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO regel_fremdrift (regel, regelversjon, siste_xid)
        VALUES ($1, $2, $3::bigint::text::xid8)
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

        upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, 100)
            .await
            .unwrap();
        assert_eq!(
            get_regel_fremdrift(&mut conn).await.unwrap(),
            vec![RegelFremdrift {
                regel: RegelId::IngenPeriodeUtenStartet,
                regelversjon: 1,
                siste_xid: 100,
            }]
        );

        assert!(
            upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, 200)
                .await
                .unwrap()
        );
        assert_eq!(
            get_regel_fremdrift(&mut conn).await.unwrap(),
            vec![RegelFremdrift {
                regel: RegelId::IngenPeriodeUtenStartet,
                regelversjon: 1,
                siste_xid: 200,
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
            upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, 100)
                .await
                .unwrap()
        );
        assert!(
            !upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, 100)
                .await
                .unwrap()
        );
        assert!(
            !upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, 99)
                .await
                .unwrap()
        );

        assert_eq!(
            get_regel_fremdrift(&mut conn).await.unwrap(),
            vec![RegelFremdrift {
                regel: RegelId::IngenPeriodeUtenStartet,
                regelversjon: 1,
                siste_xid: 100,
            }]
        );
    }

    #[tokio::test]
    async fn regler_og_versjoner_er_uavhengige_og_transaksjon_kan_rulles_tilbake() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, 100)
            .await
            .unwrap();
        upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 2, 200)
            .await
            .unwrap();
        upsert_regel_fremdrift(&mut conn, RegelId::HeleTraceSignert, 1, 300)
            .await
            .unwrap();

        let rows = get_regel_fremdrift(&mut conn).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.contains(&RegelFremdrift {
            regel: RegelId::IngenPeriodeUtenStartet,
            regelversjon: 1,
            siste_xid: 100,
        }));
        assert!(rows.contains(&RegelFremdrift {
            regel: RegelId::IngenPeriodeUtenStartet,
            regelversjon: 2,
            siste_xid: 200,
        }));
        assert!(rows.contains(&RegelFremdrift {
            regel: RegelId::HeleTraceSignert,
            regelversjon: 1,
            siste_xid: 300,
        }));
    }
}
