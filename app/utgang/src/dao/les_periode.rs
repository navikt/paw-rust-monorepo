use std::num::NonZeroU16;

use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;

use super::{periode_rad::PeriodeRad, tilstand::Tilstand};

pub async fn oppdater_periode(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: ArbeidssoekerperiodeId,
    sist_oppdatert: chrono::DateTime<chrono::Utc>,
    trenger_kontroll: bool,
    tilstand: Option<Tilstand>,
) -> Result<(), sqlx::Error> {
    let tilstand_json = tilstand
        .map(|t| serde_json::to_value(t))
        .transpose()
        .map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    sqlx::query(
        r#"UPDATE perioder
           SET sist_oppdatert = $1, trenger_kontroll = $2, tilstand = COALESCE($3, tilstand)
           WHERE id = $4"#,
    )
    .bind(sist_oppdatert.naive_utc())
    .bind(trenger_kontroll)
    .bind(&tilstand_json)
    .bind(id.0)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn hent_utdaterte_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    foer: chrono::DateTime<chrono::Utc>,
    limit: NonZeroU16,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT id, arbeidssoeker_id, identitetsnummer, stoppet, sist_oppdatert, trenger_kontroll, siste_kontroll_tidspunkt, tilstand
           FROM perioder
           WHERE sist_oppdatert < $1 AND trenger_kontroll = false
           LIMIT $2"#,
    )
    .bind(foer.naive_utc())
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await
}
