use chrono::NaiveDateTime;
use sqlx::Row;
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use types::identitetsnummer::Identitetsnummer;
use uuid::Uuid;

use super::tilstand::{Stoppet, Tilstand};

pub struct PeriodeRad {
    pub id: ArbeidssoekerperiodeId,
    pub arbeidssoeker_id: Option<ArbeidssoekerId>,
    pub identitetsnummer: Identitetsnummer,
    pub stoppet: Option<Stoppet>,
    pub sist_oppdatert: chrono::DateTime<chrono::Utc>,
    pub trenger_kontroll: bool,
    pub siste_kontroll_tidspunkt: Option<chrono::DateTime<chrono::Utc>>,
    pub tilstand: Option<Tilstand>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PeriodeRad {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let arbeidssoeker_id: Option<i64> = row.try_get("arbeidssoeker_id")?;
        let identitetsnummer: String = row.try_get("identitetsnummer")?;
        let identitetsnummer = Identitetsnummer::new(identitetsnummer)
            .ok_or_else(|| sqlx::Error::Decode("Ugyldig identitetsnummer".into()))?;
        let stoppet_json: Option<serde_json::Value> = row.try_get("stoppet")?;
        let stoppet = stoppet_json
            .map(serde_json::from_value::<Stoppet>)
            .transpose()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let sist_oppdatert: NaiveDateTime = row.try_get("sist_oppdatert")?;
        let trenger_kontroll: bool = row.try_get("trenger_kontroll")?;
        let siste_kontroll_tidspunkt: Option<NaiveDateTime> =
            row.try_get("siste_kontroll_tidspunkt")?;
        let tilstand_json: Option<serde_json::Value> = row.try_get("tilstand")?;
        let tilstand = tilstand_json
            .map(serde_json::from_value::<Tilstand>)
            .transpose()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        Ok(PeriodeRad {
            id: ArbeidssoekerperiodeId::from(id),
            arbeidssoeker_id: arbeidssoeker_id.map(ArbeidssoekerId),
            identitetsnummer,
            stoppet,
            sist_oppdatert: sist_oppdatert.and_utc(),
            trenger_kontroll,
            siste_kontroll_tidspunkt: siste_kontroll_tidspunkt.map(|t| t.and_utc()),
            tilstand,
        })
    }
}

