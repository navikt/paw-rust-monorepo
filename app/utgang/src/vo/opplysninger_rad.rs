use std::str::FromStr;

use chrono::NaiveDateTime;
use interne_hendelser::vo::Opplysning;
use sqlx::{FromRow, Row, postgres::PgRow};
use uuid::Uuid;

use crate::vo::kilde::InfoKilde;

pub struct OpplysningerRad {
    pub id: i64,
    pub periode_id: Uuid,
    pub kilde: InfoKilde,
    pub tidspunkt: chrono::DateTime<chrono::Utc>,
    pub opplysninger: Vec<Opplysning>,
}

impl FromRow<'_, PgRow> for OpplysningerRad {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: i64 = row.try_get("id")?;
        let periode_id: Uuid = row.try_get("periode_id")?;
        let kilde: InfoKilde = row.try_get("kilde").map(|k| {
            InfoKilde::from_str(k).map_err(|op| sqlx::Error::ColumnDecode {
                index: "kilde".into(),
                source: Box::new(op),
            })
        })??;
        let tidspunkt: NaiveDateTime = row.try_get("tidspunkt")?;
        let oplysninger: Vec<String> = row.try_get("opplysninger")?;
        let opplysninger: Vec<Opplysning> = oplysninger
            .into_iter()
            .filter_map(|s| {
                let opl = Opplysning::from_str(s.as_str());
                match opl {
                    Ok(o) => Some(o),
                    Err(e) => {
                        tracing::error!("Feil ved deserialisering av opplysning: {e}");
                        None
                    }
                }
            })
            .collect();
        Ok(Self {
            id,
            periode_id,
            kilde,
            tidspunkt: tidspunkt.and_utc(),
            opplysninger,
        })
    }
}
