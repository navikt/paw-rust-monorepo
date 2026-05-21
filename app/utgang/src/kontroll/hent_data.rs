use std::collections::HashSet;
use std::num::NonZeroU16;
use std::str::FromStr;

use chrono::NaiveDateTime;
use interne_hendelser::vo::{Opplysning, Opplysninger};
use sqlx::Row;
use tracing::instrument;
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use types::identitetsnummer::Identitetsnummer;
use uuid::Uuid;

pub struct PeriodeKontrollData {
    pub periode_id: ArbeidssoekerperiodeId,
    pub identitetsnummer: Identitetsnummer,
    pub gjeldende_opplysninger: Opplysninger,
    pub forrige_opplysninger: Option<Opplysninger>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PeriodeKontrollData {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let identitetsnummer: String = row.try_get("identitetsnummer")?;
        let identitetsnummer = Identitetsnummer::new(identitetsnummer)
            .ok_or_else(|| sqlx::Error::Decode("Ugyldig identitetsnummer".into()))?;

        let gjeldende: Vec<String> = row.try_get("gjeldende_opplysninger")?;
        let gjeldende_opplysninger = parse_opplysninger(gjeldende);

        let forrige: Option<Vec<String>> = row.try_get("forrige_opplysninger")?;
        let forrige_opplysninger = forrige.map(parse_opplysninger);

        Ok(Self {
            periode_id: ArbeidssoekerperiodeId::from(id),
            identitetsnummer,
            gjeldende_opplysninger,
            forrige_opplysninger,
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

#[instrument(skip(tx))]
pub async fn hent_perioder_for_kontroll(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    limit: NonZeroU16,
) -> Result<Vec<PeriodeKontrollData>, sqlx::Error> {
    sqlx::query_as::<_, PeriodeKontrollData>(
        r#"
        SELECT
            p.id,
            p.identitetsnummer,
            gjeldende.opplysninger AS gjeldende_opplysninger,
            forrige.opplysninger AS forrige_opplysninger
        FROM perioder p
        JOIN LATERAL (
            SELECT opplysninger
            FROM utgang_hendelser_logg
            WHERE periode_id = p.id AND type = 'PDL_DATA_ENDRET'
            ORDER BY timestamp DESC
            LIMIT 1
        ) gjeldende ON true
        LEFT JOIN LATERAL (
            SELECT opplysninger
            FROM utgang_hendelser_logg
            WHERE periode_id = p.id AND type = 'PDL_DATA_ENDRET'
            ORDER BY timestamp DESC
            OFFSET 1
            LIMIT 1
        ) forrige ON true
        WHERE p.trenger_kontroll = true AND p.stoppet = false
        ORDER BY p.sist_oppdatert ASC
        LIMIT $1
        "#,
    )
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await
}
