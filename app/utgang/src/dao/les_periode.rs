use super::periode_rad::PeriodeRad;

pub async fn hent_utdaterte_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    foer: chrono::DateTime<chrono::Utc>,
    limit: i64,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT id, arbeidssoeker_id, identitetsnummer, stoppet, sist_oppdatert, trenger_kontroll, siste_kontroll_tidspunkt, tilstand
           FROM perioder
           WHERE sist_oppdatert < $1 AND trenger_kontroll = false
           LIMIT $2"#,
    )
    .bind(foer.naive_utc())
    .bind(limit)
    .fetch_all(&mut **tx)
    .await
}
