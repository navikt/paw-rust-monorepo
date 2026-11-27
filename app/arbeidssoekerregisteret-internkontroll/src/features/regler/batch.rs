use crate::features::innlesing::metadata::record_metadata_id::RecordMetadataId;
use sqlx::PgConnection;

use crate::features::regler::fremdrift::Fremdrift;

/// Én melding i en batch, i den rekkefølgen regelen skal se dem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct BatchRad {
    pub record_metadata_id: RecordMetadataId,
    pub topic_id: i16,
    pub record_offset: i64,
    /// stream_time etter denne offseten: `record_timestamp_ms - LEAST(timestamp_delta_ms, 0)`.
    pub stream_ms: Option<i64>,
}

/// De neste `antall` meldingene etter `fremdrift` i partisjonen, sortert på
/// stream_time.
///
/// Hvert topic bidrar med en sammenhengende rekke offsets fra fremdriften og
/// opp. Det holder fordi stream_time aldri synker innenfor en topic-partisjon,
/// og `record_offset` avgjør ved lik tid. Da kan `LIMIT` bare kutte bort slutten
/// av et topic, aldri midten, og fremdriften kan flyttes til høyeste offset per
/// topic uten å hoppe over noe. Endres sorteringen, må dette fortsatt holde.
///
/// Meldinger uten tidsstempel arver stream_time fra raden før, eller fra
/// fremdriften hvis de er først.
pub async fn neste_batch(
    conn: &mut PgConnection,
    partisjon: i32,
    fremdrift: &[Fremdrift],
    antall: i64,
) -> Result<Vec<BatchRad>, sqlx::Error> {
    let topic_ids: Vec<i16> = fremdrift.iter().map(|f| f.topic_id).collect();
    let offsets: Vec<i64> = fremdrift.iter().map(|f| f.siste_offset).collect();
    let stream_ms: Vec<Option<i64>> = fremdrift.iter().map(|f| f.siste_stream_ms).collect();
    sqlx::query_as(
        "SELECT b.record_metadata_id, b.topic_id, b.record_offset, b.stream_ms
         FROM UNNEST($1::SMALLINT[], $2::BIGINT[], $3::BIGINT[])
              AS f(topic_id, siste_offset, siste_stream_ms)
         CROSS JOIN LATERAL (
             SELECT rm.id AS record_metadata_id,
                    rm.topic_id,
                    rm.record_offset,
                    COALESCE(
                        MAX(rm.record_timestamp_ms - LEAST(rm.timestamp_delta_ms, 0))
                            OVER (ORDER BY rm.record_offset),
                        f.siste_stream_ms
                    ) AS stream_ms
             FROM record_metadata rm
             WHERE rm.topic_id = f.topic_id
               AND rm.record_partition = $4
               AND rm.record_offset > f.siste_offset
             ORDER BY rm.record_offset
             LIMIT $5
         ) b
         ORDER BY b.stream_ms NULLS FIRST, b.topic_id, b.record_offset
         LIMIT $5",
    )
    .bind(topic_ids)
    .bind(offsets)
    .bind(stream_ms)
    .bind(partisjon)
    .bind(antall)
    .fetch_all(conn)
    .await
}
