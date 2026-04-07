use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use interne_hendelser::vo::Opplysning;
use sqlx::{FromRow, Row, postgres::PgRow};
use uuid::Uuid;

use crate::vo::kilde::InfoKilde;

pub struct KlarForKontrollRad {
    pub id: i64,
    pub opplysninger_id: i64,
    pub periode_id: Uuid,
    pub kilde: InfoKilde,
    pub tidspunkt: DateTime<Utc>,
    pub opplysninger: Vec<Opplysning>,
    pub identitetsnummer: String,
    pub arbeidssoeker_id: i64,
    pub kafka_key: i64,
    pub startet_opplysninger: Option<Vec<Opplysning>>,
    pub forrige_pdl_opplysninger: Option<Vec<Opplysning>>,
}

impl FromRow<'_, PgRow> for KlarForKontrollRad {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let id: i64 = row.try_get("id")?;
        let opplysninger_id: i64 = row.try_get("opplysninger_id")?;
        let periode_id: Uuid = row.try_get("periode_id")?;
        let kilde: InfoKilde = row.try_get("kilde").map(|k| {
            InfoKilde::from_str(k).map_err(|e| sqlx::Error::ColumnDecode {
                index: "kilde".into(),
                source: Box::new(e),
            })
        })??;
        let tidspunkt: NaiveDateTime = row.try_get("tidspunkt")?;
        let opplysninger = parse_opplysninger(row.try_get("opplysninger")?);
        let identitetsnummer: String = row.try_get("identitetsnummer")?;
        let arbeidssoeker_id: i64 = row.try_get("arbeidssoeker_id")?;
        let kafka_key: i64 = row.try_get("kafka_key")?;
        let startet_opplysninger = row
            .try_get::<Option<Vec<String>>, _>("startet_opplysninger")?
            .map(parse_opplysninger);
        let forrige_pdl_opplysninger = row
            .try_get::<Option<Vec<String>>, _>("forrige_pdl_opplysninger")?
            .map(parse_opplysninger);

        Ok(Self {
            id,
            opplysninger_id,
            periode_id,
            kilde,
            tidspunkt: tidspunkt.and_utc(),
            opplysninger,
            identitetsnummer,
            arbeidssoeker_id,
            kafka_key,
            startet_opplysninger,
            forrige_pdl_opplysninger,
        })
    }
}

fn parse_opplysninger(raw: Vec<String>) -> Vec<Opplysning> {
    raw.into_iter()
        .filter_map(|s| match Opplysning::from_str(&s) {
            Ok(o) => Some(o),
            Err(e) => {
                tracing::error!("Feil ved deserialisering av opplysning: {e}");
                None
            }
        })
        .collect()
}
