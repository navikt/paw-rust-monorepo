use chrono::NaiveDateTime;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::{
    arbeidssoeker_id::ArbeidssoekerId, arbeidssoekerperiode_id::ArbeidssoekerperiodeId,
};

pub struct PeriodeRad {
    pub id: ArbeidssoekerperiodeId,
    pub arbeidssoeker_id: Option<ArbeidssoekerId>,
    pub trenger_kontroll: bool,
    pub stoppet: bool,
    pub sist_oppdatert: chrono::DateTime<chrono::Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PeriodeRad {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let arbeidssoeker_id: Option<i64> = row.try_get("arbeidssoeker_id")?;
        let trenger_kontroll: bool = row.try_get("trenger_kontroll")?;
        let stoppet: bool = row.try_get("stoppet")?;
        let sist_oppdatert: NaiveDateTime = row.try_get("sist_oppdatert")?;
        Ok(PeriodeRad {
            id: ArbeidssoekerperiodeId::from(id),
            arbeidssoeker_id: arbeidssoeker_id.map(ArbeidssoekerId),
            trenger_kontroll,
            stoppet,
            sist_oppdatert: sist_oppdatert.and_utc(),
        })
    }
}

pub async fn skriv_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    perioder: Vec<PeriodeRad>,
) -> Result<(), sqlx::Error> {
    if perioder.is_empty() {
        return Ok(());
    }
    let mut builder = sqlx::QueryBuilder::new(
        "INSERT INTO perioder (id, arbeidssoeker_id, trenger_kontroll, stoppet, sist_oppdatert) ",
    );
    builder.push_values(perioder, |mut b, p| {
        b.push_bind(p.id.0)
            .push_bind(p.arbeidssoeker_id.map(|a| a.0))
            .push_bind(p.trenger_kontroll)
            .push_bind(p.stoppet)
            .push_bind(p.sist_oppdatert.naive_utc());
    });
    builder.push(
        " ON CONFLICT (id) DO UPDATE SET
            arbeidssoeker_id = EXCLUDED.arbeidssoeker_id,
            trenger_kontroll = EXCLUDED.trenger_kontroll,
            stoppet          = EXCLUDED.stoppet,
            sist_oppdatert   = EXCLUDED.sist_oppdatert",
    );
    builder.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn hent_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(vec![]);
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    sqlx::query_as::<_, PeriodeRad>(
        "SELECT id, arbeidssoeker_id, trenger_kontroll, stoppet, sist_oppdatert
         FROM perioder
         WHERE id = ANY($1)
           AND stoppet = false",
    )
    .bind(uuid_liste)
    .fetch_all(&mut **tx)
    .await
}

pub async fn hent_perioder_eldre_enn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    foer: chrono::DateTime<chrono::Utc>,
    limit: std::num::NonZeroU32,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    sqlx::query_as::<_, PeriodeRad>(
        "SELECT id, arbeidssoeker_id, trenger_kontroll, stoppet, sist_oppdatert
         FROM perioder
         WHERE trenger_kontroll = false AND stoppet = false AND sist_oppdatert < $1
         ORDER BY sist_oppdatert ASC
         LIMIT $2",
    )
    .bind(foer.naive_utc())
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await
}

pub async fn hent_perioder_som_trenger_kontroll(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    limit: std::num::NonZeroU32,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    sqlx::query_as::<_, PeriodeRad>(
        "SELECT id, arbeidssoeker_id, trenger_kontroll, stoppet, sist_oppdatert
         FROM perioder
         WHERE trenger_kontroll = true AND stoppet = false
         ORDER BY sist_oppdatert ASC
         LIMIT $1",
    )
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await
}
pub async fn oppdater_trenger_kontroll(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
    trenger_kontroll: bool,
) -> Result<(), sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(());
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    sqlx::query(
        "UPDATE perioder
         SET trenger_kontroll = $1
         WHERE id = ANY($2)",
    )
    .bind(trenger_kontroll)
    .bind(uuid_liste)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn oppdater_sist_oppdatert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
    sist_oppdatert: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(());
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    sqlx::query(
        "UPDATE perioder
         SET sist_oppdatert = $1
         WHERE id = ANY($2)",
    )
    .bind(sist_oppdatert.naive_utc())
    .bind(uuid_liste)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn oppdater_stoppet(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
) -> Result<(), sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(());
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    sqlx::query(
        "UPDATE perioder
         SET stoppet = true
         WHERE id = ANY($1)",
    )
    .bind(uuid_liste)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
