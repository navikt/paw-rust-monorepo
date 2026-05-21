use std::collections::HashSet;
use std::str::FromStr;

use chrono::NaiveDateTime;
use interne_hendelser::vo::{Opplysning, Opplysninger};
use sqlx::Row;
use tracing::instrument;
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;
use uuid::Uuid;

use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "kontroll_status_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KontrollStatusType {
    Godkjent,
    Avvist,
    Ukjent,
}

pub struct PeriodeRad {
    pub id: ArbeidssoekerperiodeId,
    pub arbeidssoeker_id: Option<ArbeidssoekerId>,
    pub trenger_kontroll: bool,
    pub stoppet: bool,
    pub sist_oppdatert: chrono::DateTime<chrono::Utc>,
    pub identitetsnummer: Identitetsnummer,
    pub initielle_opplysninger: Option<Opplysninger>,
    pub gjeldende_opplysninger: Option<Opplysninger>,
    pub gjeldende_tidspunkt: Option<chrono::DateTime<chrono::Utc>>,
    pub forrige_opplysninger: Option<Opplysninger>,
    pub forrige_tidspunkt: Option<chrono::DateTime<chrono::Utc>>,
    pub siste_status: KontrollStatusType,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PeriodeRad {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let arbeidssoeker_id: Option<i64> = row.try_get("arbeidssoeker_id")?;
        let trenger_kontroll: bool = row.try_get("trenger_kontroll")?;
        let stoppet: bool = row.try_get("stoppet")?;
        let sist_oppdatert: NaiveDateTime = row.try_get("sist_oppdatert")?;
        let identitetsnummer: String = row.try_get("identitetsnummer")?;
        let identitetsnummer = Identitetsnummer::new(identitetsnummer)
            .ok_or_else(|| sqlx::Error::Decode("Ugyldig identitetsnummer".into()))?;
        let initielle_opplysninger: Option<Vec<String>> =
            row.try_get("initielle_opplysninger")?;
        let gjeldende_opplysninger: Option<Vec<String>> =
            row.try_get("gjeldende_opplysninger")?;
        let gjeldende_tidspunkt: Option<NaiveDateTime> = row.try_get("gjeldende_tidspunkt")?;
        let forrige_opplysninger: Option<Vec<String>> = row.try_get("forrige_opplysninger")?;
        let forrige_tidspunkt: Option<NaiveDateTime> = row.try_get("forrige_tidspunkt")?;
        let siste_status: KontrollStatusType = row.try_get("siste_status")?;

        Ok(PeriodeRad {
            id: ArbeidssoekerperiodeId::from(id),
            arbeidssoeker_id: arbeidssoeker_id.map(ArbeidssoekerId),
            trenger_kontroll,
            stoppet,
            sist_oppdatert: sist_oppdatert.and_utc(),
            identitetsnummer,
            initielle_opplysninger: initielle_opplysninger.map(parse_opplysninger),
            gjeldende_opplysninger: gjeldende_opplysninger.map(parse_opplysninger),
            gjeldende_tidspunkt: gjeldende_tidspunkt.map(|t| t.and_utc()),
            forrige_opplysninger: forrige_opplysninger.map(parse_opplysninger),
            forrige_tidspunkt: forrige_tidspunkt.map(|t| t.and_utc()),
            siste_status,
        })
    }
}

fn parse_opplysninger(strings: Vec<String>) -> Opplysninger {
    Opplysninger(
        strings
            .into_iter()
            .filter_map(|s| Opplysning::from_str(&s).ok())
            .collect::<HashSet<_>>(),
    )
}

#[instrument(skip(tx, perioder))]
pub async fn skriv_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    perioder: &[PeriodeRad],
) -> Result<(), sqlx::Error> {
    if perioder.is_empty() {
        return Ok(());
    }
    let mut builder = sqlx::QueryBuilder::new(
        "INSERT INTO perioder (id, arbeidssoeker_id, identitetsnummer, trenger_kontroll, stoppet, sist_oppdatert, initielle_opplysninger, siste_status) ",
    );
    builder.push_values(perioder, |mut b, p| {
        let initielle = p
            .initielle_opplysninger
            .as_ref()
            .map(|o| o.to_string_vector());
        b.push_bind(p.id.0)
            .push_bind(p.arbeidssoeker_id.map(|a| a.0))
            .push_bind(p.identitetsnummer.as_ref())
            .push_bind(p.trenger_kontroll)
            .push_bind(p.stoppet)
            .push_bind(p.sist_oppdatert.naive_utc())
            .push_bind(initielle)
            .push_bind(&p.siste_status);
    });
    builder.push(
        " ON CONFLICT (id) DO UPDATE SET
            arbeidssoeker_id = EXCLUDED.arbeidssoeker_id,
            trenger_kontroll = EXCLUDED.trenger_kontroll,
            stoppet          = EXCLUDED.stoppet,
            sist_oppdatert   = EXCLUDED.sist_oppdatert,
            initielle_opplysninger = COALESCE(perioder.initielle_opplysninger, EXCLUDED.initielle_opplysninger)",
    );
    builder.build().execute(&mut **tx).await?;
    Ok(())
}

#[instrument(skip(tx, periode_ider))]
pub async fn hent_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(vec![]);
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    sqlx::query_as::<_, PeriodeRad>(
        "SELECT * FROM perioder WHERE id = ANY($1) AND stoppet = false",
    )
    .bind(uuid_liste)
    .fetch_all(&mut **tx)
    .await
}

#[instrument(skip(tx))]
pub async fn hent_perioder_eldre_enn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    foer: chrono::DateTime<chrono::Utc>,
    limit: std::num::NonZeroU16,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    sqlx::query_as::<_, PeriodeRad>(
        "SELECT * FROM perioder
         WHERE trenger_kontroll = false AND stoppet = false AND sist_oppdatert < $1
         ORDER BY sist_oppdatert ASC
         LIMIT $2",
    )
    .bind(foer.naive_utc())
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await
}

#[instrument(skip(tx))]
pub async fn hent_perioder_som_trenger_kontroll(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    limit: std::num::NonZeroU16,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    sqlx::query_as::<_, PeriodeRad>(
        "SELECT * FROM perioder
         WHERE trenger_kontroll = true AND stoppet = false
         ORDER BY sist_oppdatert ASC
         LIMIT $1",
    )
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await
}

#[instrument(skip(tx, periode_ider))]
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
        "UPDATE perioder SET trenger_kontroll = $1 WHERE id = ANY($2)",
    )
    .bind(trenger_kontroll)
    .bind(uuid_liste)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[instrument(skip(tx, periode_ider))]
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
        "UPDATE perioder SET sist_oppdatert = $1 WHERE id = ANY($2)",
    )
    .bind(sist_oppdatert.naive_utc())
    .bind(uuid_liste)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[instrument(skip(tx, periode_ider))]
pub async fn oppdater_stoppet(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
) -> Result<(), sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(());
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    sqlx::query("UPDATE perioder SET stoppet = true WHERE id = ANY($1)")
        .bind(uuid_liste)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[instrument(skip(tx))]
pub async fn oppdater_pdl_opplysninger(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_id: &ArbeidssoekerperiodeId,
    nye_opplysninger: &Opplysninger,
    tidspunkt: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    let opl_vec = nye_opplysninger.to_string_vector();
    sqlx::query(
        r#"UPDATE perioder SET
            forrige_opplysninger = gjeldende_opplysninger,
            forrige_tidspunkt = gjeldende_tidspunkt,
            gjeldende_opplysninger = $1,
            gjeldende_tidspunkt = $2,
            trenger_kontroll = true,
            sist_oppdatert = $2
         WHERE id = $3"#,
    )
    .bind(&opl_vec)
    .bind(tidspunkt.naive_utc())
    .bind(periode_id.0)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[instrument(skip(tx))]
pub async fn oppdater_kontroll_status(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_id: &ArbeidssoekerperiodeId,
    status: &KontrollStatusType,
    tidspunkt: chrono::DateTime<chrono::Utc>,
    opplysninger: Option<&Opplysninger>,
) -> Result<(), sqlx::Error> {
    let opl_vec = opplysninger.map(|o| o.to_string_vector());
    sqlx::query(
        "UPDATE perioder SET siste_status = $1, trenger_kontroll = false WHERE id = $2",
    )
    .bind(status)
    .bind(periode_id.0)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO kontroll_status_logg (periode_id, status, tidspunkt, opplysninger) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode_id.0)
    .bind(status)
    .bind(tidspunkt.naive_utc())
    .bind(opl_vec)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
