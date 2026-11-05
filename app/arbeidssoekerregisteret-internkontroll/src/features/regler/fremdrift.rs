use sqlx::PgConnection;

use crate::features::regler::{regel_katalog::RegelId, regelversjon::Regelversjon};

/// Hvor langt en regelversjon har kommet i én topic-partisjon. Én rad i `regel_fremdrift`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct Fremdrift {
    pub regel: RegelId,
    pub regelversjon: Regelversjon,
    pub topic_id: i16,
    pub topic_partition: i32,
    pub siste_offset: i64,
    pub siste_stream_ms: Option<i64>,
}

/// Fremdrift for alle topics regelen har lest i partisjonen.
/// Topics uten rad er ikke lest ennå.
pub async fn hent_fremdrift(
    conn: &mut PgConnection,
    regel: RegelId,
    versjon: Regelversjon,
    partisjon: i32,
) -> Result<Vec<Fremdrift>, sqlx::Error> {
    sqlx::query_as(
        "SELECT regel, regelversjon, topic_id, topic_partition, siste_offset, siste_stream_ms
         FROM regel_fremdrift
         WHERE regel = $1 AND regelversjon = $2 AND topic_partition = $3
         ORDER BY topic_id",
    )
    .bind(regel)
    .bind(versjon)
    .bind(partisjon)
    .fetch_all(conn)
    .await
}

/// Lagrer fremdrift. Flytter aldri `siste_offset` bakover.
pub async fn oppdater_fremdrift(
    conn: &mut PgConnection,
    fremdrift: &[Fremdrift],
) -> Result<(), sqlx::Error> {
    let regler: Vec<RegelId> = fremdrift.iter().map(|f| f.regel).collect();
    let versjoner: Vec<Regelversjon> = fremdrift.iter().map(|f| f.regelversjon).collect();
    let partisjoner: Vec<i32> = fremdrift.iter().map(|f| f.topic_partition).collect();
    let topic_ids: Vec<i16> = fremdrift.iter().map(|f| f.topic_id).collect();
    let offsets: Vec<i64> = fremdrift.iter().map(|f| f.siste_offset).collect();
    let stream_ms: Vec<Option<i64>> = fremdrift.iter().map(|f| f.siste_stream_ms).collect();
    sqlx::query(
        "INSERT INTO regel_fremdrift
             (regel, regelversjon, topic_partition, topic_id, siste_offset, siste_stream_ms)
         SELECT * FROM UNNEST(
             $1::SMALLINT[], $2::INTEGER[], $3::INTEGER[], $4::SMALLINT[], $5::BIGINT[], $6::BIGINT[]
         )
         ON CONFLICT (regel, regelversjon, topic_id, topic_partition) DO UPDATE
             SET siste_offset = EXCLUDED.siste_offset,
                 siste_stream_ms = EXCLUDED.siste_stream_ms
             WHERE regel_fremdrift.siste_offset < EXCLUDED.siste_offset",
    )
    .bind(regler)
    .bind(versjoner)
    .bind(partisjoner)
    .bind(topic_ids)
    .bind(offsets)
    .bind(stream_ms)
    .execute(conn)
    .await?;
    Ok(())
}
