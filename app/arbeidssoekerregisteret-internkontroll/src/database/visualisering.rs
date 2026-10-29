use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::database::signature::SignatureStatus;

use crate::regler::regel_katalog::RegelId;

/// Øvre grense for tellingen av records en regel ligger bak. Tellingen går
/// over indeksen på `record_derived.xid`, men et stort etterslep skal ikke
/// gjøre siden treg. `etter` lik grensen betyr «minst så mange».
///
/// `ORDER BY xid` i tellingen er nødvendig: uten den kan Postgres velge å lese
/// tabellen sekvensielt og håpe på tidlige treff, men radene ligger i
/// xid-rekkefølge, så den leser alle eldre rader først.
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
                    ORDER BY rd.xid
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
    /// Kafka-tidsstempel for siste record uten trace_id (høyeste xid).
    pub siste_timestamp_ms: Option<i64>,
    /// `timestamp_delta_ms` for siste record uten trace_id.
    pub siste_timestamp_delta_ms: Option<i64>,
}

/// Antall records i `record_derived` uten trace_id, per topic-navn, med
/// tidsstempel og tidsdelta for siste slike record (høyeste xid).
/// Topics uten slike records er ikke med.
///
/// Telling og oppslag går per topic over den delvise indeksen
/// `record_derived_uten_trace_id_idx (topic_id, xid) WHERE trace_id IS NULL`,
/// og bare siste record per topic slås opp i `kafka_record`.
#[tracing::instrument(skip(conn))]
pub async fn hent_mangler_trace_id_per_topic(
    conn: &mut PgConnection,
) -> Result<Vec<ManglerTraceId>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT t.name AS topic,
               antall.antall,
               kr.record_timestamp_ms AS siste_timestamp_ms,
               siste.timestamp_delta_ms AS siste_timestamp_delta_ms
        FROM topic t
        CROSS JOIN LATERAL (
            SELECT count(*) AS antall FROM record_derived
            WHERE topic_id = t.id AND trace_id IS NULL
        ) antall
        CROSS JOIN LATERAL (
            SELECT record_partition, record_offset, timestamp_delta_ms
            FROM record_derived
            WHERE topic_id = t.id AND trace_id IS NULL
            ORDER BY xid DESC
            LIMIT 1
        ) siste
        JOIN kafka_record kr
          ON kr.topic_id = t.id
         AND kr.record_partition = siste.record_partition
         AND kr.record_offset = siste.record_offset
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
                siste_timestamp_ms: row.try_get("siste_timestamp_ms")?,
                siste_timestamp_delta_ms: row.try_get("siste_timestamp_delta_ms")?,
            })
        })
        .collect()
}

/// Antall signaturfeil som vises.
pub const ANTALL_SIGNATURFEIL: i64 = 20;

/// Én record som feilet signaturvalidering. Inneholder ikke key eller value,
/// som kan ha personopplysninger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signaturfeil {
    pub topic: String,
    pub record_partition: i32,
    pub record_offset: i64,
    pub record_timestamp_ms: Option<i64>,
    pub signature_status: SignatureStatus,
    pub signing_key_id: Option<String>,
    pub trace_id: Option<Uuid>,
}

/// De siste `ANTALL_SIGNATURFEIL` records som feilet signaturvalidering,
/// nyeste først (høyeste xid).
///
/// Feilet betyr alle statuser unntatt `valid` og `unsigned_before_required`
/// (signatur var ikke påkrevd ennå). `no_record_timestamp` og `technical_error`
/// er med selv om valideringen ikke ble fullført. Filteret står som fast tekst
/// og må være identisk med predikatet til `record_derived_signaturfeil_idx`,
/// ellers bruker ikke Postgres den delvise indeksen.
#[tracing::instrument(skip(conn))]
pub async fn hent_siste_signaturfeil(
    conn: &mut PgConnection,
) -> Result<Vec<Signaturfeil>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT t.name AS topic, rd.record_partition, rd.record_offset,
               kr.record_timestamp_ms, rd.signature_status, rd.signing_key_id, rd.trace_id
        FROM record_derived rd
        JOIN topic t ON t.id = rd.topic_id
        JOIN kafka_record kr
          ON kr.topic_id = rd.topic_id
         AND kr.record_partition = rd.record_partition
         AND kr.record_offset = rd.record_offset
        WHERE rd.signature_status NOT IN ('valid', 'unsigned_before_required')
        ORDER BY rd.xid DESC
        LIMIT $1
        "#,
    )
    .bind(ANTALL_SIGNATURFEIL)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(Signaturfeil {
                topic: row.try_get("topic")?,
                record_partition: row.try_get("record_partition")?,
                record_offset: row.try_get("record_offset")?,
                record_timestamp_ms: row.try_get("record_timestamp_ms")?,
                signature_status: row.try_get("signature_status")?,
                signing_key_id: row.try_get("signing_key_id")?,
                trace_id: row.try_get("trace_id")?,
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

    /// Begge spørringene må kunne bruke sin delvise indeks. Uten den leser
    /// Postgres hele `record_derived`, som tar minutter i prod.
    #[tokio::test]
    async fn spoerringene_bruker_de_delvise_indeksene() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        sqlx::query("SET enable_seqscan = off")
            .execute(&mut *conn)
            .await
            .unwrap();

        async fn plan(conn: &mut PgConnection, explain: &'static str) -> String {
            let linjer: Vec<String> = sqlx::query_scalar(explain)
                .fetch_all(&mut *conn)
                .await
                .unwrap();
            linjer.join("\n")
        }
        let signaturfeil = plan(
            &mut conn,
            "EXPLAIN SELECT xid FROM record_derived \
             WHERE signature_status NOT IN ('valid', 'unsigned_before_required') \
             ORDER BY xid DESC LIMIT 20",
        )
        .await;
        assert!(
            signaturfeil.contains("record_derived_signaturfeil_idx"),
            "{signaturfeil}"
        );
        let uten_trace = plan(
            &mut conn,
            "EXPLAIN SELECT xid FROM record_derived \
             WHERE topic_id = 1 AND trace_id IS NULL ORDER BY xid DESC LIMIT 1",
        )
        .await;
        assert!(
            uten_trace.contains("record_derived_uten_trace_id_idx"),
            "{uten_trace}"
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
        sqlx::query(
            "UPDATE kafka_record SET record_timestamp_ms = 2000 WHERE topic_id = $1 AND record_offset = 1",
        )
        .bind(a)
        .execute(&mut *conn)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE record_derived SET timestamp_delta_ms = 500 WHERE topic_id = $1 AND record_offset = 1",
        )
        .bind(a)
        .execute(&mut *conn)
        .await
        .unwrap();

        assert_eq!(
            hent_mangler_trace_id_per_topic(&mut conn).await.unwrap(),
            vec![
                ManglerTraceId {
                    topic: "topic-a".to_string(),
                    antall: 2,
                    siste_timestamp_ms: Some(2000),
                    siste_timestamp_delta_ms: Some(500),
                },
                ManglerTraceId {
                    topic: "topic-b".to_string(),
                    antall: 1,
                    siste_timestamp_ms: Some(1000),
                    siste_timestamp_delta_ms: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn signaturfeil_viser_bare_feilet_signatur_nyeste_foerst() {
        use crate::database::kafka_record::{
            KafkaRecordRow, KafkaTimestampType, insert_kafka_record,
        };
        use crate::database::payload::PayloadStatus;
        use crate::database::record_derived::{RecordDerived, insert_record_derived};
        use crate::database::signature::Signature;

        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &["topic-a"]).await.unwrap();
        let topic_id = load_topic_ids(&mut conn)
            .await
            .unwrap()
            .get("topic-a")
            .unwrap();

        assert!(hent_siste_signaturfeil(&mut conn).await.unwrap().is_empty());

        let key = || "key-a".to_string();
        let signaturer = [
            Signature::Valid {
                signing_key_id: key(),
            },
            Signature::Missing,
            Signature::UnsignedBeforeRequired,
            Signature::CryptographicMismatch {
                signing_key_id: key(),
            },
            Signature::TechnicalError,
            Signature::NoRecordTimestamp,
            Signature::UnknownKey,
        ];
        for (offset, signature) in signaturer.into_iter().enumerate() {
            let offset = offset as i64;
            insert_kafka_record(
                &mut conn,
                &KafkaRecordRow {
                    topic_id,
                    record_partition: 0,
                    record_offset: offset,
                    record_timestamp_type: KafkaTimestampType::CreateTime,
                    record_timestamp_ms: Some(1_000 + offset),
                    record_key: None,
                    record_value: None,
                    header_keys: vec![],
                    header_values: vec![],
                },
            )
            .await
            .unwrap();
            insert_record_derived(
                &mut conn,
                &RecordDerived {
                    topic_id,
                    record_partition: 0,
                    record_offset: offset,
                    signature,
                    timestamp_delta_ms: None,
                    trace_id: Some(Uuid::from_u128(offset as u128)),
                    payload_status: PayloadStatus::Ok,
                    meldingstype: None,
                    periode_id: None,
                    arbeidssoeker_id: None,
                    derived_at: chrono::DateTime::from_timestamp_millis(10_000).unwrap(),
                },
            )
            .await
            .unwrap();
        }

        let feil = hent_siste_signaturfeil(&mut conn).await.unwrap();
        assert_eq!(
            feil.iter().map(|f| f.record_offset).collect::<Vec<_>>(),
            vec![6, 5, 4, 3, 1]
        );
        assert_eq!(
            feil[3],
            Signaturfeil {
                topic: "topic-a".to_string(),
                record_partition: 0,
                record_offset: 3,
                record_timestamp_ms: Some(1_003),
                signature_status: SignatureStatus::SignatureMismatch,
                signing_key_id: Some("key-a".to_string()),
                trace_id: Some(Uuid::from_u128(3)),
            }
        );
    }
}
