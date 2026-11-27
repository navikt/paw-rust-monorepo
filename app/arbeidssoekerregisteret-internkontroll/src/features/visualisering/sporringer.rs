//! Lesespørringer for visualiseringen. Ingen av dem returnerer `periode_id`
//! eller `arbeidssoeker_id`. Bare `hent_detaljmeldinger` returnerer
//! personopplysninger, og de pseudonymiseres før visning. Se modulkommentaren
//! i `mod.rs` før du legger til en spørring som gjør det.

use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::features::innlesing::metadata::{
    record_metadata_id::RecordMetadataId, signature::SignatureStatus, utfoert::UtfoertAvType,
};
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
    pub ajour: bool,
}

/// Fremdrift per regelversjon. `meldinger_bak` er summen av HWM minus
/// `siste_offset` over partisjonene regelen har startet på.
///
/// `eldste_stream_ms` tar bare med partisjoner med meldinger igjen å lese,
/// eller der HWM er ukjent.
/// En partisjon uten ny trafikk står ellers med stream-tiden til sin siste
/// melding, som kan være uker gammel, og regelen ville sett ut til å henge
/// etter selv om den er ajour. `ajour` er sann når HWM er kjent og nådd i
/// alle partisjonene.
pub async fn hent_regel_status(
    conn: &mut PgConnection,
    hwm_version: i16,
) -> Result<Vec<RegelStatus>, sqlx::Error> {
    sqlx::query_as(
        "SELECT rf.regel, rf.regelversjon,
                COUNT(*) AS partisjoner,
                MIN(rf.siste_stream_ms) FILTER (WHERE h.hwm IS NULL OR h.hwm > rf.siste_offset) AS eldste_stream_ms,
                COALESCE(SUM(GREATEST(h.hwm - rf.siste_offset, 0)), 0)::BIGINT AS meldinger_bak,
                (SELECT COUNT(*) FROM modne_funn f
                 WHERE f.regel = rf.regel AND f.regelversjon = rf.regelversjon) AS antall_funn,
                BOOL_AND(h.hwm IS NOT NULL AND h.hwm <= rf.siste_offset) AS ajour
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
    pub record_metadata_id: RecordMetadataId,
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
        "SELECT f.record_metadata_id, f.regel, f.regelversjon, f.kode, f.alvorlighet,
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

#[derive(Debug, FromRow)]
pub struct FunnDetaljer {
    pub kode: i16,
    pub alvorlighet: Alvorlighet,
    pub meldinger: Vec<RecordMetadataId>,
    pub detaljer: sqlx::types::Json<serde_json::Map<String, serde_json::Value>>,
}

/// Ett gjeldende funn, slått opp på hele nøkkelen.
pub async fn hent_funn_detaljer(
    conn: &mut PgConnection,
    regel: RegelId,
    regelversjon: Regelversjon,
    record_metadata_id: RecordMetadataId,
) -> Result<Option<FunnDetaljer>, sqlx::Error> {
    sqlx::query_as(
        "SELECT kode, alvorlighet, meldinger, detaljer FROM funn
         WHERE regel = $1 AND regelversjon = $2 AND record_metadata_id = $3",
    )
    .bind(regel)
    .bind(regelversjon)
    .bind(record_metadata_id)
    .fetch_optional(conn)
    .await
}

/// En melding i et funn, med personopplysninger. Skal bare vises etter
/// pseudonymisering, se `pseudonym.rs`.
#[derive(FromRow)]
pub struct DetaljMelding {
    pub id: RecordMetadataId,
    pub topic: String,
    pub record_partition: i32,
    pub record_offset: i64,
    pub record_timestamp_ms: Option<i64>,
    pub meldingstype: Option<String>,
    pub kilde: Option<String>,
    pub trace_id: Option<Uuid>,
    pub identitetsnummer: Option<String>,
    pub utfoert_tidspunkt_ms: Option<i64>,
    pub utfoert_av_type: Option<UtfoertAvType>,
    pub utfoert_av_id: Option<String>,
}

impl std::fmt::Debug for DetaljMelding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetaljMelding")
            .field("id", &self.id)
            .field("topic", &self.topic)
            .field("record_partition", &self.record_partition)
            .field("record_offset", &self.record_offset)
            .field("utfoert_av_type", &self.utfoert_av_type)
            .finish_non_exhaustive()
    }
}

/// Meldingene til et funn. `meldinger` skal komme fra `funn.meldinger`, aldri
/// fra requesten. Meldinger som er slettet, mangler i svaret.
pub async fn hent_detaljmeldinger(
    conn: &mut PgConnection,
    meldinger: &[RecordMetadataId],
) -> Result<Vec<DetaljMelding>, sqlx::Error> {
    sqlx::query_as(
        "SELECT rm.id, t.name AS topic, rm.record_partition, rm.record_offset,
                rm.record_timestamp_ms, rm.meldingstype::TEXT AS meldingstype,
                rm.kilde::TEXT AS kilde, rm.trace_id,
                rm.identitetsnummer, rm.utfoert_tidspunkt_ms, rm.utfoert_av_type,
                rm.utfoert_av_id
         FROM record_metadata rm
         JOIN topic t ON t.id = rm.topic_id
         WHERE rm.id = ANY($1)",
    )
    .bind(meldinger)
    .fetch_all(conn)
    .await
}
