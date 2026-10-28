use sqlx::{PgConnection, Row};

use crate::regler::regel_katalog::RegelId;

/// Øvre grense for tellingen av records en regel ligger bak. Tellingen går
/// over indeksen på `record_derived.xid`, men et stort etterslep skal ikke
/// gjøre siden treg. `etter` lik grensen betyr «minst så mange».
pub const MAKS_ETTERSLEP: i64 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegelStatus {
    pub regel: RegelId,
    pub regelversjon: i32,
    pub siste_xid: i64,
    /// Antall records i `record_derived` med xid over `siste_xid`, maks `MAKS_ETTERSLEP`.
    pub etter: i64,
    pub antall_funn: i64,
}

/// Fremdrift, etterslep og antall funn per regel og regelversjon.
#[tracing::instrument(skip(conn))]
pub async fn hent_regel_status(conn: &mut PgConnection) -> Result<Vec<RegelStatus>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT f.regel,
               f.regelversjon,
               f.siste_xid::text::bigint AS siste_xid,
               (SELECT count(*) FROM (
                    SELECT 1 FROM record_derived rd
                    WHERE rd.xid > f.siste_xid
                    LIMIT $1
               ) etterslep) AS etter,
               (SELECT count(*) FROM funn
                WHERE funn.regel = f.regel AND funn.regelversjon = f.regelversjon) AS antall_funn
        FROM regel_fremdrift f
        ORDER BY f.regel, f.regelversjon
        "#,
    )
    .bind(MAKS_ETTERSLEP)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(RegelStatus {
                regel: row.try_get("regel")?,
                regelversjon: row.try_get("regelversjon")?,
                siste_xid: row.try_get("siste_xid")?,
                etter: row.try_get("etter")?,
                antall_funn: row.try_get("antall_funn")?,
            })
        })
        .collect()
}

/// Høyeste xid i `record_derived`, eller `None` når tabellen er tom.
#[tracing::instrument(skip(conn))]
pub async fn hent_max_xid(conn: &mut PgConnection) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT xid::text::bigint FROM record_derived
        ORDER BY xid DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *conn)
    .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManglerTraceId {
    pub topic: String,
    pub antall: i64,
}

/// Antall records i `record_derived` uten trace_id, per topic-navn.
/// Topics uten slike records er ikke med.
#[tracing::instrument(skip(conn))]
pub async fn hent_mangler_trace_id_per_topic(
    conn: &mut PgConnection,
) -> Result<Vec<ManglerTraceId>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT t.name AS topic, count(*) AS antall
        FROM record_derived rd
        JOIN topic t ON t.id = rd.topic_id
        WHERE rd.trace_id IS NULL
        GROUP BY t.name
        ORDER BY t.name
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(ManglerTraceId {
                topic: row.try_get("topic")?,
                antall: row.try_get("antall")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::funn::upsert_funn;
    use crate::database::record_derived::tests::skriv_record;
    use crate::database::regel_fremdrift::upsert_regel_fremdrift;
    use crate::database::topic::{load_topic_ids, register_topics};
    use crate::database::trace::upsert_trace;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use uuid::Uuid;

    #[tokio::test]
    async fn regelstatus_viser_etterslep_og_funn_per_regel() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        assert!(hent_regel_status(&mut conn).await.unwrap().is_empty());
        assert_eq!(hent_max_xid(&mut conn).await.unwrap(), None);

        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        let topic_id = load_topic_ids(&mut conn)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();
        let xid1 = skriv_record(&mut conn, topic_id, 0).await;
        let _xid2 = skriv_record(&mut conn, topic_id, 1).await;
        let xid3 = skriv_record(&mut conn, topic_id, 2).await;

        upsert_regel_fremdrift(&mut conn, RegelId::HeleTraceSignert, 1, xid1)
            .await
            .unwrap();
        upsert_regel_fremdrift(&mut conn, RegelId::IngenPeriodeUtenStartet, 1, xid3)
            .await
            .unwrap();
        for i in 1..=2 {
            let trace_id = Uuid::from_u128(i);
            upsert_trace(&mut conn, trace_id, 1_000).await.unwrap();
            upsert_funn(&mut conn, trace_id, RegelId::HeleTraceSignert, 1, 1)
                .await
                .unwrap();
        }

        assert_eq!(hent_max_xid(&mut conn).await.unwrap(), Some(xid3));
        assert_eq!(
            hent_regel_status(&mut conn).await.unwrap(),
            vec![
                RegelStatus {
                    regel: RegelId::HeleTraceSignert,
                    regelversjon: 1,
                    siste_xid: xid1,
                    etter: 2,
                    antall_funn: 2,
                },
                RegelStatus {
                    regel: RegelId::IngenPeriodeUtenStartet,
                    regelversjon: 1,
                    siste_xid: xid3,
                    etter: 0,
                    antall_funn: 0,
                },
            ]
        );
    }

    #[tokio::test]
    async fn teller_records_uten_trace_id_per_topic() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        assert!(
            hent_mangler_trace_id_per_topic(&mut conn)
                .await
                .unwrap()
                .is_empty()
        );

        register_topics(&mut conn, &["topic-a", "topic-b", "topic-c"])
            .await
            .unwrap();
        let ids = load_topic_ids(&mut conn).await.unwrap();
        let a = ids.get("topic-a").unwrap();
        let b = ids.get("topic-b").unwrap();
        skriv_record(&mut conn, a, 0).await;
        skriv_record(&mut conn, a, 1).await;
        skriv_record(&mut conn, b, 0).await;
        sqlx::query("UPDATE record_derived SET trace_id = $1 WHERE topic_id = $2")
            .bind(Uuid::from_u128(1))
            .bind(b)
            .execute(&mut *conn)
            .await
            .unwrap();
        skriv_record(&mut conn, b, 1).await;

        assert_eq!(
            hent_mangler_trace_id_per_topic(&mut conn).await.unwrap(),
            vec![
                ManglerTraceId {
                    topic: "topic-a".to_string(),
                    antall: 2
                },
                ManglerTraceId {
                    topic: "topic-b".to_string(),
                    antall: 1
                },
            ]
        );
    }
}
