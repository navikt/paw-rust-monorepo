use std::num::NonZeroU16;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::vo::opplysninger_rad::OpplysningerRad;
use crate::vo::periode_metadata_rad::PeriodeMetadata;
use crate::vo::periode_rad::PeriodeRad;
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

pub async fn hent_sist_oppdatert_foer(
    tx: &mut Transaction<'_, Postgres>,
    tidspunkt: &chrono::DateTime<chrono::Utc>,
    status: &Status,
    limit: &NonZeroU16,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    let res: Vec<PeriodeRad> = sqlx::query_as::<_, PeriodeRad>(
        r#"
        select * from periode 
        where 
            sist_oppdatert_timestamp < $1 and
            sist_oppdatert_status = $2 
            order by sist_oppdatert_timestamp ASC limit $3
        "#,
    )
    .bind(tidspunkt)
    .bind(status.to_string())
    .bind(limit.get() as i64)
    .fetch_all(&mut **tx)
    .await?;
    Ok(res)
}

pub async fn hent_periode_metadata(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &Uuid,
) -> Result<PeriodeMetadata, sqlx::Error> {
    let res: PeriodeMetadata = sqlx::query_as::<_, PeriodeMetadata>(
        r#"
        select * from periode_metadata where periode_id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(res)
}
