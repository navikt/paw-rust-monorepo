use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use interne_hendelser::vo::BrukerType;
use sqlx::{FromRow, Row, postgres::PgRow};
use uuid::Uuid;

use crate::vo::status::Status;

pub struct PeriodeMedMetadataRad {
    pub id: Uuid,
    pub periode_aktiv: bool,
    pub periode_startet_timestamp: DateTime<Utc>,
    pub periode_starter_brukertype: BrukerType,
    pub periode_avsluttet_timestamp: Option<DateTime<Utc>>,
    pub periode_avsluttet_brukertype: Option<BrukerType>,
    pub sist_oppdatert_timestamp: DateTime<Utc>,
    pub sist_oppdatert_status: Status,
    pub identitetsnummer: String,
    pub arbeidssoeker_id: i64,
    pub kafka_key: i64,
}

impl FromRow<'_, PgRow> for PeriodeMedMetadataRad {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let periode_aktiv: bool = row.try_get("periode_aktiv")?;
        let periode_startet_timestamp: NaiveDateTime = row.try_get("periode_startet_timestamp")?;
        let periode_starter_brukertype =
            BrukerType::from_str(row.try_get("periode_startet_brukertype")?).map_err(|e| {
                sqlx::Error::ColumnDecode {
                    index: "periode_startet_brukertype".into(),
                    source: Box::new(e),
                }
            })?;
        let periode_avsluttet_timestamp: Option<NaiveDateTime> =
            row.try_get("periode_avsluttet_timestamp")?;
        let periode_avsluttet_brukertype: Option<BrukerType> = row
            .try_get::<Option<String>, _>("periode_avsluttet_brukertype")?
            .map(|s| BrukerType::from_str(&s))
            .transpose()
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "periode_avsluttet_brukertype".into(),
                source: Box::new(e),
            })?;
        let sist_oppdatert_timestamp: NaiveDateTime = row.try_get("sist_oppdatert_timestamp")?;
        let sist_oppdatert_status = Status::from_str(row.try_get("sist_oppdatert_status")?)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "sist_oppdatert_status".into(),
                source: Box::new(e),
            })?;
        let identitetsnummer: String = row.try_get("identitetsnummer")?;
        let arbeidssoeker_id: i64 = row.try_get("arbeidssoeker_id")?;
        let kafka_key: i64 = row.try_get("kafka_key")?;

        Ok(PeriodeMedMetadataRad {
            id,
            periode_aktiv,
            periode_startet_timestamp: periode_startet_timestamp.and_utc(),
            periode_starter_brukertype,
            periode_avsluttet_timestamp: periode_avsluttet_timestamp.map(|dt| dt.and_utc()),
            periode_avsluttet_brukertype,
            sist_oppdatert_timestamp: sist_oppdatert_timestamp.and_utc(),
            sist_oppdatert_status,
            identitetsnummer,
            arbeidssoeker_id,
            kafka_key,
        })
    }
}
