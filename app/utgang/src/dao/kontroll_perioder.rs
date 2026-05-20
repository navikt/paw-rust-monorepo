use std::collections::HashSet;
use std::str::FromStr;

use chrono::NaiveDateTime;
use sqlx::Row;
use tracing::instrument;
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;

use interne_hendelser::vo::{Opplysning, Opplysninger};

pub struct PdlEndring {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub opplysninger: Option<Opplysninger>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PdlEndring {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let timestamp: NaiveDateTime = row.try_get("timestamp")?;
        let opplysninger: Option<Vec<String>> = row.try_get("opplysninger")?;
        let opplysninger = opplysninger.map(|strings| {
            Opplysninger(
                strings
                    .into_iter()
                    .filter_map(|s| Opplysning::from_str(&s).ok())
                    .collect::<HashSet<_>>(),
            )
        });
        Ok(Self {
            timestamp: timestamp.and_utc(),
            opplysninger,
        })
    }
}

#[instrument(skip(tx))]
pub async fn hent_siste_pdl_endringer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_id: &ArbeidssoekerperiodeId,
) -> Result<Vec<PdlEndring>, sqlx::Error> {
    sqlx::query_as::<_, PdlEndring>(
        r#"
        SELECT timestamp, opplysninger
        FROM utgang_hendelser_logg
        WHERE periode_id = $1
          AND type = 'PDL_DATA_ENDRET'
        ORDER BY timestamp DESC
        LIMIT 2
        "#,
    )
    .bind(periode_id.0)
    .fetch_all(&mut **tx)
    .await
}
