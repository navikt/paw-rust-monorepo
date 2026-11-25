//! Oppslag reglene deler. Alle oppslag i historikk ser bare på meldinger
//! regelen allerede har behandlet, se [`crate::features::regler::regel::Regel::vurder`].

use sqlx::PgConnection;
use uuid::Uuid;

use crate::features::innlesing::metadata::{payload::Meldingstype, signature::SignatureStatus};
use crate::features::regler::fremdrift::Fremdrift;

/// Feltene fra `record_metadata` reglene trenger om meldingen de vurderer.
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
    record_metadata_id: i64,
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

/// Hvor langt regelen har kommet i `topic_id`. -1 når topicet ikke er lest.
fn siste_offset(sett: &[Fremdrift], topic_id: i16) -> i64 {
    sett.iter()
        .find(|f| f.topic_id == topic_id)
        .map_or(-1, |f| f.siste_offset)
}

/// Har regelen sett en melding på `topic_id` i `partisjon` med `periode_id`
/// og en av `meldingstyper`? Tom `meldingstyper` godtar alle typer.
pub async fn har_sett_periode(
    conn: &mut PgConnection,
    sett: &[Fremdrift],
    topic_id: i16,
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
    .bind(topic_id)
    .bind(partisjon)
    .bind(siste_offset(sett, topic_id))
    .bind(meldingstyper)
    .fetch_one(conn)
    .await
}

/// Meldingstypen til siste melding før `melding` på samme topic og partisjon
/// med samme `arbeidssoeker_id` og en av `meldingstyper`. `None` når det ikke
/// finnes noen, eller meldingen mangler `arbeidssoeker_id`.
pub async fn forrige_for_arbeidssoeker(
    conn: &mut PgConnection,
    sett: &[Fremdrift],
    melding: &Melding,
    meldingstyper: &[Meldingstype],
) -> Result<Option<Meldingstype>, sqlx::Error> {
    let Some(arbeidssoeker_id) = melding.arbeidssoeker_id else {
        return Ok(None);
    };
    let grense = siste_offset(sett, melding.topic_id).min(melding.record_offset - 1);
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
