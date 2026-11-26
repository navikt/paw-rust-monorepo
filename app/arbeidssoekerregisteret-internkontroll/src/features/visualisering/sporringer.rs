//! Lesespørringer for visualiseringen. Ingen av dem returnerer `periode_id`
//! eller `arbeidssoeker_id`. Se modulkommentaren i `mod.rs` før du legger til
//! en spørring som gjør det.

use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::features::innlesing::metadata::signature::SignatureStatus;
use crate::features::regler::{
    alvorlighet::Alvorlighet, regel_katalog::RegelId, regelversjon::Regelversjon,
};

pub const MAKS_FUNN_I_LISTE: i64 = 100;
pub const MAKS_SIGNATURFEIL_I_LISTE: i64 = 50;

/// Lesetransaksjon med tidsgrense. HTMX spør på nytt med faste intervaller,
/// så en treg spørring skal feile i stedet for å hope seg opp. Rulles
/// tilbake når den droppes.
pub async fn lesetransaksjon(
    pg_pool: &PgPool,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = pg_pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

#[derive(Debug, FromRow)]
pub struct Toppfelt {
    pub stream_ms: Option<i64>,
    pub kritiske_funn: i64,
    pub advarsler: i64,
    pub signaturfeil: i64,
}

pub async fn hent_toppfelt(conn: &mut PgConnection) -> Result<Toppfelt, sqlx::Error> {
    sqlx::query_as(
        "SELECT
            (SELECT MAX(timestamp_ms) FROM stream_time) AS stream_ms,
            (SELECT COUNT(*) FROM modne_funn WHERE alvorlighet = 'kritisk') AS kritiske_funn,
            (SELECT COUNT(*) FROM modne_funn WHERE alvorlighet = 'advarsel') AS advarsler,
            (SELECT COUNT(*) FROM record_metadata
             WHERE signature_status NOT IN ('valid', 'unsigned_before_required')) AS signaturfeil",
    )
    .fetch_one(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct RegelStatus {
    pub regel: RegelId,
    pub regelversjon: Regelversjon,
    pub partisjoner: i64,
    pub eldste_stream_ms: Option<i64>,
    pub meldinger_bak: i64,
    pub antall_funn: i64,
}

/// Fremdrift per regelversjon. `meldinger_bak` er summen av HWM minus
/// `siste_offset` over partisjonene regelen har startet på.
pub async fn hent_regel_status(
    conn: &mut PgConnection,
    hwm_version: i16,
) -> Result<Vec<RegelStatus>, sqlx::Error> {
    sqlx::query_as(
        "SELECT rf.regel, rf.regelversjon,
                COUNT(*) AS partisjoner,
                MIN(rf.siste_stream_ms) AS eldste_stream_ms,
                COALESCE(SUM(GREATEST(h.hwm - rf.siste_offset, 0)), 0)::BIGINT AS meldinger_bak,
                (SELECT COUNT(*) FROM modne_funn f
                 WHERE f.regel = rf.regel AND f.regelversjon = rf.regelversjon) AS antall_funn
         FROM regel_fremdrift rf
         JOIN topic t ON t.id = rf.topic_id
         LEFT JOIN hwm h ON h.version = $1 AND h.topic = t.name
                        AND h.partition = rf.topic_partition
         GROUP BY rf.regel, rf.regelversjon
         ORDER BY rf.regel, rf.regelversjon",
    )
    .bind(hwm_version)
    .fetch_all(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct TopicStatus {
    pub topic: String,
    pub partisjoner: i64,
    pub eldste_stream_ms: Option<i64>,
    pub nyeste_stream_ms: Option<i64>,
}

pub async fn hent_topic_status(conn: &mut PgConnection) -> Result<Vec<TopicStatus>, sqlx::Error> {
    sqlx::query_as(
        "SELECT t.name AS topic,
                COUNT(s.topic_partition) AS partisjoner,
                MIN(s.timestamp_ms) AS eldste_stream_ms,
                MAX(s.timestamp_ms) AS nyeste_stream_ms
         FROM topic t
         LEFT JOIN stream_time s ON s.topic_id = t.id
         GROUP BY t.name
         ORDER BY t.name",
    )
    .fetch_all(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct FunnAntall {
    pub regel: RegelId,
    pub regelversjon: Regelversjon,
    pub alvorlighet: Alvorlighet,
    pub antall: i64,
}

pub async fn hent_funn_antall(conn: &mut PgConnection) -> Result<Vec<FunnAntall>, sqlx::Error> {
    sqlx::query_as(
        "SELECT regel, regelversjon, alvorlighet, COUNT(*) AS antall
         FROM modne_funn
         GROUP BY regel, regelversjon, alvorlighet
         ORDER BY regel, regelversjon, alvorlighet DESC",
    )
    .fetch_all(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct FunnRad {
    pub regel: RegelId,
    pub regelversjon: Regelversjon,
    pub kode: i16,
    pub alvorlighet: Alvorlighet,
    pub topic: String,
    pub record_partition: i32,
    pub record_offset: i64,
    pub record_timestamp_ms: Option<i64>,
    pub meldingstype: Option<String>,
    pub trace_id: Option<Uuid>,
}

/// De nyeste funnene, mest alvorlige først.
pub async fn hent_funn(
    conn: &mut PgConnection,
    regel: Option<RegelId>,
    alvorlighet: Option<Alvorlighet>,
) -> Result<Vec<FunnRad>, sqlx::Error> {
    sqlx::query_as(
        "SELECT f.regel, f.regelversjon, f.kode, f.alvorlighet,
                t.name AS topic, rm.record_partition, rm.record_offset,
                rm.record_timestamp_ms, rm.meldingstype::TEXT AS meldingstype, rm.trace_id
         FROM modne_funn f
         JOIN record_metadata rm ON rm.id = f.record_metadata_id
         JOIN topic t ON t.id = rm.topic_id
         WHERE ($1::SMALLINT IS NULL OR f.regel = $1)
           AND ($2::alvorlighet IS NULL OR f.alvorlighet = $2)
         ORDER BY f.alvorlighet DESC, f.record_metadata_id DESC
         LIMIT $3",
    )
    .bind(regel)
    .bind(alvorlighet)
    .bind(MAKS_FUNN_I_LISTE)
    .fetch_all(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct SignaturAntall {
    pub topic: String,
    pub signature_status: SignatureStatus,
    pub antall: i64,
}

pub async fn hent_signatur_antall(
    conn: &mut PgConnection,
) -> Result<Vec<SignaturAntall>, sqlx::Error> {
    sqlx::query_as(
        "SELECT t.name AS topic, rm.signature_status, COUNT(*) AS antall
         FROM record_metadata rm
         JOIN topic t ON t.id = rm.topic_id
         WHERE rm.signature_status NOT IN ('valid', 'unsigned_before_required')
         GROUP BY t.name, rm.signature_status
         ORDER BY t.name, rm.signature_status",
    )
    .fetch_all(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct Signaturfeil {
    pub topic: String,
    pub record_partition: i32,
    pub record_offset: i64,
    pub record_timestamp_ms: Option<i64>,
    pub signature_status: SignatureStatus,
    pub key_id: Option<String>,
    pub trace_id: Option<Uuid>,
}

pub async fn hent_siste_signaturfeil(
    conn: &mut PgConnection,
) -> Result<Vec<Signaturfeil>, sqlx::Error> {
    sqlx::query_as(
        "SELECT t.name AS topic, rm.record_partition, rm.record_offset,
                rm.record_timestamp_ms, rm.signature_status, sk.key_id, rm.trace_id
         FROM record_metadata rm
         JOIN topic t ON t.id = rm.topic_id
         LEFT JOIN signing_key sk ON sk.id = rm.signing_key_id
         WHERE rm.signature_status NOT IN ('valid', 'unsigned_before_required')
         ORDER BY rm.id DESC
         LIMIT $1",
    )
    .bind(MAKS_SIGNATURFEIL_I_LISTE)
    .fetch_all(conn)
    .await
}

#[derive(Debug, FromRow)]
pub struct ManglerTraceId {
    pub topic: String,
    pub antall: i64,
    pub siste_timestamp_ms: Option<i64>,
}

pub async fn hent_mangler_trace_id(
    conn: &mut PgConnection,
) -> Result<Vec<ManglerTraceId>, sqlx::Error> {
    sqlx::query_as(
        "SELECT t.name AS topic, COUNT(*) AS antall,
                MAX(rm.record_timestamp_ms) AS siste_timestamp_ms
         FROM record_metadata rm
         JOIN topic t ON t.id = rm.topic_id
         WHERE rm.trace_id IS NULL
         GROUP BY t.name
         ORDER BY t.name",
    )
    .fetch_all(conn)
    .await
}
