use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{FromRow, PgConnection, Row};
use uuid::Uuid;

use crate::features::innlesing::metadata::arbeidssoeker_id::ArbeidssoekerId;
use crate::features::innlesing::metadata::kilde::{Kilde, System};
use crate::features::innlesing::metadata::payload::{Meldingstype, PayloadStatus};
use crate::features::innlesing::metadata::signature::Signature;
use crate::features::innlesing::record::kafka_record::KafkaTimestampType;

#[derive(Clone, PartialEq, Eq)]
pub struct RecordMetadata {
    pub topic_id: i16,
    pub record_partition: i32,
    pub record_offset: i64,
    pub record_timestamp_type: KafkaTimestampType,
    pub record_timestamp_ms: Option<i64>,
    pub signature: Signature,
    pub timestamp_delta_ms: Option<i64>,
    pub trace_id: Option<Uuid>,
    pub kilde: Option<Kilde>,
    pub system: Option<System>,
    pub payload_status: PayloadStatus,
    pub meldingstype: Option<Meldingstype>,
    pub periode_id: Option<Uuid>,
    pub arbeidssoeker_id: Option<ArbeidssoekerId>,
    pub derived_at: DateTime<Utc>,
}

impl std::fmt::Debug for RecordMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordMetadata")
            .field("topic_id", &self.topic_id)
            .field("record_partition", &self.record_partition)
            .field("record_offset", &self.record_offset)
            .field("record_timestamp_type", &self.record_timestamp_type)
            .field("record_timestamp_ms", &self.record_timestamp_ms)
            .field("signature", &self.signature)
            .field("timestamp_delta_ms", &self.timestamp_delta_ms)
            .field("trace_id", &self.trace_id)
            .field("kilde", &self.kilde)
            .field("system", &self.system)
            .field("payload_status", &self.payload_status)
            .field("meldingstype", &self.meldingstype)
            .field("derived_at", &self.derived_at)
            .finish()
    }
}

/// Expects `signing_key.key_id` selected as `signing_key_id`, since the table
/// only stores the reference to `signing_key`.
impl FromRow<'_, PgRow> for RecordMetadata {
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
            record_timestamp_type: row.try_get("record_timestamp_type")?,
            record_timestamp_ms: row.try_get("record_timestamp_ms")?,
            signature,
            timestamp_delta_ms: row.try_get("timestamp_delta_ms")?,
            trace_id: row.try_get("trace_id")?,
            kilde: row.try_get("kilde")?,
            system: row.try_get("system")?,
            payload_status: row.try_get("payload_status")?,
            meldingstype: row.try_get("meldingstype")?,
            periode_id: row.try_get("periode_id")?,
            arbeidssoeker_id: row.try_get("arbeidssoeker_id")?,
            derived_at: row.try_get("derived_at")?,
        })
    }
}

/// Returns the id in `signing_key` for `key_id`, creating the row the first time.
async fn signing_key_id(conn: &mut PgConnection, key_id: &str) -> Result<i16, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        WITH ny AS (
            INSERT INTO signing_key (key_id) VALUES ($1)
            ON CONFLICT (key_id) DO NOTHING
            RETURNING id
        )
        SELECT id FROM ny
        UNION ALL
        SELECT id FROM signing_key WHERE key_id = $1
        LIMIT 1
        "#,
    )
    .bind(key_id)
    .fetch_one(&mut *conn)
    .await
}

/// Writes the derived values for a record and returns the new row id.
/// A record is derived once, so writing it again is an error.
#[tracing::instrument(skip(conn, record))]
pub async fn insert_record_metadata(
    conn: &mut PgConnection,
    record: &RecordMetadata,
) -> Result<i64, sqlx::Error> {
    let signing_key_id = match record.signature.signing_key_id() {
        Some(key_id) => Some(i64::from(signing_key_id(conn, key_id).await?)),
        None => None,
    };
    sqlx::query_scalar(
        r#"
        INSERT INTO record_metadata (
            topic_id, record_partition, record_offset,
            record_timestamp_type, record_timestamp_ms,
            signature_status, signing_key_id,
            timestamp_delta_ms, trace_id, kilde, system,
            payload_status, meldingstype, periode_id, arbeidssoeker_id, derived_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id
        "#,
    )
    .bind(record.topic_id)
    .bind(record.record_partition)
    .bind(record.record_offset)
    .bind(record.record_timestamp_type)
    .bind(record.record_timestamp_ms)
    .bind(record.signature.status())
    .bind(signing_key_id)
    .bind(record.timestamp_delta_ms)
    .bind(record.trace_id)
    .bind(record.kilde)
    .bind(record.system)
    .bind(record.payload_status)
    .bind(record.meldingstype)
    .bind(record.periode_id)
    .bind(record.arbeidssoeker_id)
    .bind(record.derived_at)
    .fetch_one(&mut *conn)
    .await
}
