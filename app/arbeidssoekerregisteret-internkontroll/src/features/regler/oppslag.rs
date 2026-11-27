use crate::features::innlesing::metadata::record_metadata_id::RecordMetadataId;
use sqlx::PgConnection;
use uuid::Uuid;

use paw_kafka::topics::Topic;

use crate::features::innlesing::metadata::{payload::Meldingstype, signature::SignatureStatus};
use crate::features::regler::fremdrift::Fremdrift;

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct Melding {
    pub topic_id: i16,
    pub record_partition: i32,
    pub record_offset: i64,
    pub meldingstype: Option<Meldingstype>,
    pub periode_id: Option<Uuid>,
    pub arbeidssoeker_id: Option<i64>,
    pub signature_status: SignatureStatus,
}

pub async fn hent_melding(
    conn: &mut PgConnection,
    record_metadata_id: RecordMetadataId,
) -> Result<Melding, sqlx::Error> {
    sqlx::query_as(
        "SELECT topic_id, record_partition, record_offset, meldingstype, periode_id,
                arbeidssoeker_id, signature_status
         FROM record_metadata WHERE id = $1",
    )
    .bind(record_metadata_id)
    .fetch_one(conn)
    .await
}

pub struct Sett<'a> {
    topics: &'a [(Topic, i16)],
    fremdrift: &'a [Fremdrift],
}

impl<'a> Sett<'a> {
    pub fn ny(topics: &'a [(Topic, i16)], fremdrift: &'a [Fremdrift]) -> Self {
        Self { topics, fremdrift }
    }

    pub fn topic_id(&self, topic: &Topic) -> i16 {
        self.topics
            .iter()
            .find(|(t, _)| t == topic)
            .expect("topic mangler i regelens topics()")
            .1
    }

    pub fn siste_offset(&self, topic_id: i16) -> i64 {
        self.fremdrift
            .iter()
            .find(|f| f.topic_id == topic_id)
            .map_or(-1, |f| f.siste_offset)
    }
}

pub async fn har_sett_periode(
    conn: &mut PgConnection,
    sett: &Sett<'_>,
    topic: &Topic,
    partisjon: i32,
    periode_id: Uuid,
    meldingstyper: &[Meldingstype],
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM record_metadata
             WHERE periode_id = $1 AND topic_id = $2 AND record_partition = $3
               AND record_offset <= $4
               AND (cardinality($5::meldingstype[]) = 0 OR meldingstype = ANY($5))
         )",
    )
    .bind(periode_id)
    .bind(sett.topic_id(topic))
    .bind(partisjon)
    .bind(sett.siste_offset(sett.topic_id(topic)))
    .bind(meldingstyper)
    .fetch_one(conn)
    .await
}

pub async fn forrige_for_arbeidssoeker(
    conn: &mut PgConnection,
    sett: &Sett<'_>,
    melding: &Melding,
    meldingstyper: &[Meldingstype],
) -> Result<Option<Meldingstype>, sqlx::Error> {
    let Some(arbeidssoeker_id) = melding.arbeidssoeker_id else {
        return Ok(None);
    };
    let grense = sett
        .siste_offset(melding.topic_id)
        .min(melding.record_offset - 1);
    sqlx::query_scalar(
        "SELECT meldingstype FROM record_metadata
         WHERE arbeidssoeker_id = $1 AND topic_id = $2 AND record_partition = $3
           AND record_offset <= $4 AND meldingstype = ANY($5)
         ORDER BY record_offset DESC
         LIMIT 1",
    )
    .bind(arbeidssoeker_id)
    .bind(melding.topic_id)
    .bind(melding.record_partition)
    .bind(grense)
    .bind(meldingstyper)
    .fetch_optional(conn)
    .await
}
