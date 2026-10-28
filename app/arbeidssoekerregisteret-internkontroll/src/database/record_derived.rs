use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{FromRow, PgConnection, Row};
use std::num::NonZeroU32;
use uuid::Uuid;

use crate::database::payload::{Meldingstype, PayloadStatus};
use crate::database::signature::Signature;

/// Values derived from one Kafka record, as written to `record_derived`.
/// `xid` is set by the database to the writing transaction.
#[derive(Clone, PartialEq, Eq)]
pub struct RecordDerived {
    pub topic_id: i16,
    pub record_partition: i32,
    pub record_offset: i64,
    pub signature: Signature,
    pub timestamp_delta_ms: Option<i64>,
    pub trace_id: Option<Uuid>,
    pub payload_status: PayloadStatus,
    pub meldingstype: Option<Meldingstype>,
    pub periode_id: Option<Uuid>,
    pub arbeidssoeker_id: Option<i64>,
    pub derived_at: DateTime<Utc>,
}

impl std::fmt::Debug for RecordDerived {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordDerived")
            .field("topic_id", &self.topic_id)
            .field("record_partition", &self.record_partition)
            .field("record_offset", &self.record_offset)
            .field("signature", &self.signature)
            .field("timestamp_delta_ms", &self.timestamp_delta_ms)
            .field("trace_id", &self.trace_id)
            .field("payload_status", &self.payload_status)
            .field("meldingstype", &self.meldingstype)
            .field("derived_at", &self.derived_at)
            .finish()
    }
}

impl FromRow<'_, PgRow> for RecordDerived {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let signature = Signature::from_columns(
            row.try_get("signature_status")?,
            row.try_get("signing_key_id")?,
        )
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        Ok(Self {
            topic_id: row.try_get("topic_id")?,
            record_partition: row.try_get("record_partition")?,
            record_offset: row.try_get("record_offset")?,
            signature,
            timestamp_delta_ms: row.try_get("timestamp_delta_ms")?,
            trace_id: row.try_get("trace_id")?,
            payload_status: row.try_get("payload_status")?,
            meldingstype: row.try_get("meldingstype")?,
            periode_id: row.try_get("periode_id")?,
            arbeidssoeker_id: row.try_get("arbeidssoeker_id")?,
            derived_at: row.try_get("derived_at")?,
        })
    }
}

/// Writes the derived values for a record.
/// A record is derived once, so writing it again is an error.
#[tracing::instrument(skip(conn, record))]
pub async fn insert_record_derived(
    conn: &mut PgConnection,
    record: &RecordDerived,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO record_derived (
            topic_id, record_partition, record_offset,
            signature_status, signing_key_id,
            timestamp_delta_ms, trace_id, payload_status, meldingstype, periode_id, arbeidssoeker_id, derived_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(record.topic_id)
    .bind(record.record_partition)
    .bind(record.record_offset)
    .bind(record.signature.status())
    .bind(record.signature.signing_key_id())
    .bind(record.timestamp_delta_ms)
    .bind(record.trace_id)
    .bind(record.payload_status)
    .bind(record.meldingstype)
    .bind(record.periode_id)
    .bind(record.arbeidssoeker_id)
    .bind(record.derived_at)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

#[cfg(test)]
pub async fn get_record_derived(
    conn: &mut PgConnection,
    topic_id: i16,
    record_partition: i32,
    record_offset: i64,
) -> Result<Option<RecordDerived>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT topic_id, record_partition, record_offset,
               signature_status, signing_key_id,
               timestamp_delta_ms, trace_id, payload_status, meldingstype, periode_id, arbeidssoeker_id, derived_at
        FROM record_derived
        WHERE topic_id = $1 AND record_partition = $2 AND record_offset = $3
        "#,
    )
    .bind(topic_id)
    .bind(record_partition)
    .bind(record_offset)
    .fetch_optional(&mut *conn)
    .await
}

/// Get record with xid + all records where trace_id matches tracie_id of this record and xid <= the given xid, in xid order.
/// Empty if the record with the given xid does not exist or has no trace_id.
#[tracing::instrument(skip(conn))]
pub async fn get_at_xid_including_preceding_trace_records(
    conn: &mut PgConnection,
    xid: i64,
) -> Result<Vec<RecordDerived>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT rd.topic_id, rd.record_partition, rd.record_offset,
               rd.signature_status, rd.signing_key_id,
               rd.timestamp_delta_ms, rd.trace_id, rd.payload_status, rd.meldingstype,
               rd.periode_id, rd.arbeidssoeker_id, rd.derived_at
        FROM record_derived m
        JOIN record_derived rd
          ON rd.trace_id = m.trace_id
         AND rd.xid <= m.xid
        WHERE m.xid = $1::bigint::text::xid8
        ORDER BY rd.xid
        "#,
    )
    .bind(xid)
    .fetch_all(&mut *conn)
    .await
}

/// The xids of up to `limit` records after `etter_xid`, in xid order. Empty
/// when there are none yet. Only finished transactions are visible: rows from
/// an earlier xid can still commit after a later one, and the marker must never
/// skip them. xid8 values are bigint, since sqlx has no xid8 type.
#[tracing::instrument(skip(conn))]
pub async fn get_neste_record_derived_xids(
    conn: &mut PgConnection,
    etter_xid: i64,
    limit: NonZeroU32,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT xid::text::bigint
        FROM record_derived as rd
        WHERE rd.xid > $1::bigint::text::xid8
          AND rd.xid < pg_snapshot_xmin(pg_current_snapshot())
        ORDER BY rd.xid
        LIMIT $2
        "#,
    )
    .bind(etter_xid)
    .bind(i64::from(limit.get()))
    .fetch_all(&mut *conn)
    .await
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::database::kafka_record::{KafkaRecordRow, KafkaTimestampType, insert_kafka_record};
    use crate::database::signature::tests::alle_signaturer;
    use crate::database::topic::{load_topic_ids, register_topics};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgConnection;

    #[tokio::test]
    async fn record_derived_skrives_en_gang_og_leses_tilbake() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();

        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        let topic_id = load_topic_ids(&mut conn)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();
        for offset in [0, 1] {
            let row = KafkaRecordRow {
                topic_id,
                record_partition: 0,
                record_offset: offset,
                record_timestamp_type: KafkaTimestampType::CreateTime,
                record_timestamp_ms: Some(1_000),
                record_key: None,
                record_value: Some(b"value"),
                header_keys: vec![],
                header_values: vec![],
            };
            insert_kafka_record(&mut conn, &row).await.unwrap();
        }

        let derived_at = DateTime::from_timestamp_millis(10_000).unwrap();
        let first = RecordDerived {
            topic_id,
            record_partition: 0,
            record_offset: 0,
            signature: Signature::Valid {
                signing_key_id: "key-a".to_string(),
            },
            timestamp_delta_ms: None,
            trace_id: Some(Uuid::from_u128(1)),
            payload_status: PayloadStatus::Ok,
            meldingstype: Some(Meldingstype::HendelseStartet),
            periode_id: Some(Uuid::from_u128(7)),
            arbeidssoeker_id: Some(42),
            derived_at,
        };
        insert_record_derived(&mut conn, &first).await.unwrap();
        let second = RecordDerived {
            record_offset: 1,
            ..first.clone()
        };
        insert_record_derived(&mut conn, &second).await.unwrap();

        let stored = get_record_derived(&mut conn, topic_id, 0, 0)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored, first);
        let stored_second = get_record_derived(&mut conn, topic_id, 0, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored_second, second);

        assert!(
            insert_record_derived(&mut conn, &first).await.is_err(),
            "a record must not be derived twice"
        );

        let rejected = sqlx::query(
            "UPDATE record_derived SET signature_status = 'valid', signing_key_id = '' WHERE record_offset = 0",
        )
        .execute(&pool)
        .await;
        assert!(rejected.is_err(), "CHECK should reject an empty key ID");
    }

    #[tokio::test]
    async fn check_avviser_feil_kombinasjon_av_status_og_nokkel() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        let topic_id = load_topic_ids(&mut conn)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();
        let row = KafkaRecordRow {
            topic_id,
            record_partition: 0,
            record_offset: 0,
            record_timestamp_type: KafkaTimestampType::CreateTime,
            record_timestamp_ms: Some(1_000),
            record_key: None,
            record_value: Some(b"value"),
            header_keys: vec![],
            header_values: vec![],
        };
        insert_kafka_record(&mut conn, &row).await.unwrap();

        for signature in alle_signaturer() {
            let wrong_key = match signature.signing_key_id() {
                Some(_) => None,
                None => Some("key-a"),
            };
            let rejected = sqlx::query(
                "INSERT INTO record_derived (topic_id, record_partition, record_offset,
                     signature_status, signing_key_id, derived_at)
                 VALUES ($1, 0, 0, $2, $3, now())",
            )
            .bind(topic_id)
            .bind(signature.status())
            .bind(wrong_key)
            .execute(&pool)
            .await;
            assert!(
                rejected.is_err(),
                "CHECK skal avvise {:?} med nøkkel-ID {wrong_key:?}",
                signature.status()
            );
        }
    }

    #[tokio::test]
    async fn henter_records_i_en_trace_til_og_med_xid_i_skriverekkefoelge() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        let topic_id = load_topic_ids(&mut conn)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();
        let trace_a = Uuid::from_u128(1);
        let trace_b = Uuid::from_u128(2);

        let mut skrevet = Vec::new();
        for (offset, trace_id) in [
            (2, Some(trace_a)),
            (0, Some(trace_b)),
            (1, Some(trace_a)),
            (3, None),
        ] {
            let row = KafkaRecordRow {
                topic_id,
                record_partition: 0,
                record_offset: offset,
                record_timestamp_type: KafkaTimestampType::CreateTime,
                record_timestamp_ms: Some(1_000),
                record_key: None,
                record_value: None,
                header_keys: vec![],
                header_values: vec![],
            };
            insert_kafka_record(&mut conn, &row).await.unwrap();
            let derived = RecordDerived {
                topic_id,
                record_partition: 0,
                record_offset: offset,
                signature: Signature::Missing,
                timestamp_delta_ms: None,
                trace_id,
                payload_status: PayloadStatus::Ok,
                meldingstype: None,
                periode_id: None,
                arbeidssoeker_id: None,
                derived_at: DateTime::from_timestamp_millis(10_000).unwrap(),
            };
            insert_record_derived(&mut conn, &derived).await.unwrap();
            skrevet.push(derived);
        }

        let xid = |offset: i64| {
            sqlx::query_scalar::<_, i64>(
                "SELECT xid::text::bigint FROM record_derived WHERE record_offset = $1",
            )
            .bind(offset)
            .fetch_one(&pool)
        };
        let siste_xid = xid(3).await.unwrap();
        let trace_b_xid = xid(0).await.unwrap();

        assert_eq!(
            get_at_xid_including_preceding_trace_records(&mut conn, trace_b_xid)
                .await
                .unwrap(),
            vec![skrevet[1].clone()]
        );

        let offsets = |records: Vec<RecordDerived>| {
            records.iter().map(|r| r.record_offset).collect::<Vec<_>>()
        };

        let trace_til = |offset: i64| {
            let pool = pool.clone();
            async move {
                let xid = sqlx::query_scalar::<_, i64>(
                    "SELECT xid::text::bigint FROM record_derived WHERE record_offset = $1",
                )
                .bind(offset)
                .fetch_one(&pool)
                .await
                .unwrap();
                let mut conn = pool.acquire().await.unwrap();
                offsets(
                    get_at_xid_including_preceding_trace_records(&mut conn, xid)
                        .await
                        .unwrap(),
                )
            }
        };
        assert_eq!(trace_til(1).await, vec![2, 1]);
        assert_eq!(trace_til(2).await, vec![2]);
        assert_eq!(trace_til(0).await, vec![0]);
        assert_eq!(
            trace_til(3).await,
            Vec::<i64>::new(),
            "ingen trace_id gir ingen rader"
        );
        assert!(
            get_at_xid_including_preceding_trace_records(&mut conn, siste_xid + 1_000)
                .await
                .unwrap()
                .is_empty()
        );
    }

    pub(crate) async fn skriv_record(conn: &mut PgConnection, topic_id: i16, offset: i64) -> i64 {
        let row = KafkaRecordRow {
            topic_id,
            record_partition: 0,
            record_offset: offset,
            record_timestamp_type: KafkaTimestampType::CreateTime,
            record_timestamp_ms: Some(1_000),
            record_key: None,
            record_value: None,
            header_keys: vec![],
            header_values: vec![],
        };
        insert_kafka_record(conn, &row).await.unwrap();
        let derived = RecordDerived {
            topic_id,
            record_partition: 0,
            record_offset: offset,
            signature: Signature::Missing,
            timestamp_delta_ms: None,
            trace_id: None,
            payload_status: PayloadStatus::Ok,
            meldingstype: None,
            periode_id: None,
            arbeidssoeker_id: None,
            derived_at: DateTime::from_timestamp_millis(10_000).unwrap(),
        };
        insert_record_derived(conn, &derived).await.unwrap();
        sqlx::query_scalar("SELECT xid::text::bigint FROM record_derived WHERE record_offset = $1")
            .bind(offset)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    fn limit(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    async fn neste_xids_naar_ferdig(
        pool: &sqlx::PgPool,
        etter_xid: i64,
        limit: NonZeroU32,
        antall: usize,
    ) -> Vec<i64> {
        let mut xids = Vec::new();
        for _ in 0..50 {
            let mut conn = pool.acquire().await.unwrap();
            xids = get_neste_record_derived_xids(&mut conn, etter_xid, limit)
                .await
                .unwrap();
            if xids.len() >= antall {
                break;
            }
            drop(conn);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        xids
    }

    #[tokio::test]
    async fn neste_xids_hentes_i_rekkefoelge_opp_til_limit() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        let topic_id = load_topic_ids(&mut conn)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();

        assert!(
            get_neste_record_derived_xids(&mut conn, 0, limit(10))
                .await
                .unwrap()
                .is_empty()
        );

        let foerste = skriv_record(&mut conn, topic_id, 0).await;
        let andre = skriv_record(&mut conn, topic_id, 1).await;
        let tredje = skriv_record(&mut conn, topic_id, 2).await;

        assert_eq!(
            neste_xids_naar_ferdig(&pool, 0, limit(10), 3).await,
            vec![foerste, andre, tredje]
        );
        assert_eq!(
            neste_xids_naar_ferdig(&pool, 0, limit(2), 2).await,
            vec![foerste, andre]
        );
        assert_eq!(
            neste_xids_naar_ferdig(&pool, foerste, limit(1), 1).await,
            vec![andre]
        );
        assert!(
            get_neste_record_derived_xids(&mut conn, tredje, limit(10))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn neste_xids_venter_paa_eldre_aapen_transaksjon() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut conn_setup = pool.acquire().await.unwrap();
        register_topics(&mut conn_setup, &["topic-a"])
            .await
            .unwrap();
        let topic_id = load_topic_ids(&mut conn_setup)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();
        drop(conn_setup);

        let mut aapen = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_current_xact_id()")
            .execute(&mut *aapen)
            .await
            .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let senere = skriv_record(&mut conn, topic_id, 0).await;
        assert!(
            get_neste_record_derived_xids(&mut conn, 0, limit(10))
                .await
                .unwrap()
                .is_empty(),
            "rader over en åpen transaksjon skal ikke hentes"
        );

        aapen.rollback().await.unwrap();
        assert_eq!(
            neste_xids_naar_ferdig(&pool, 0, limit(10), 1).await,
            vec![senere]
        );
    }
}
