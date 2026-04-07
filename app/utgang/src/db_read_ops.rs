use std::num::NonZeroU16;

use sqlx::{Postgres, Transaction};
use tracing::instrument;
use uuid::Uuid;

use crate::vo::opplysninger_rad::OpplysningerRad;
use crate::vo::periode_med_metadata_rad::PeriodeMedMetadataRad;
use crate::vo::status::Status;

pub async fn hent_opplysninger(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
    antall: i64,
) -> Result<Vec<OpplysningerRad>, sqlx::Error> {
    let res: Vec<OpplysningerRad> = sqlx::query_as::<_, OpplysningerRad>(
        r#"
        select * from opplysninger where periode_id = $1 order by tidspunkt desc limit $2
        "#,
    )
    .bind(periode_id)
    .bind(antall)
    .fetch_all(&mut **tx)
    .await?;
    Ok(res)
}

#[instrument(skip(tx))]
pub async fn hent_sist_oppdatert_foer_med_metadata(
    tx: &mut Transaction<'_, Postgres>,
    tidspunkt: &chrono::DateTime<chrono::Utc>,
    status: &[Status],
    limit: &NonZeroU16,
) -> Result<Vec<PeriodeMedMetadataRad>, sqlx::Error> {
    let status_str_vec: Vec<String> = status.iter().map(|s| s.to_string()).collect();
    let res: Vec<PeriodeMedMetadataRad> = sqlx::query_as::<_, PeriodeMedMetadataRad>(
        r#"
        select p.*, pm.identitetsnummer, pm.arbeidssoeker_id, pm.kafka_key
        from periode p
        inner join periode_metadata pm on p.id = pm.periode_id
        where
            p.periode_avsluttet_timestamp is null and
            p.sist_oppdatert_timestamp < $1 and
            p.sist_oppdatert_status = ANY($2)
        order by p.sist_oppdatert_timestamp asc limit $3
        "#,
    )
    .bind(tidspunkt)
    .bind(status_str_vec)
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await?;
    Ok(res)
}
