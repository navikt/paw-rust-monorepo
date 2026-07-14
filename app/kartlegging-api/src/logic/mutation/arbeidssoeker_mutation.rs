use crate::model::dao::arbeidssoeker;
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::sort::SortOrder;
use sqlx::{Postgres, Transaction};

pub async fn lagre_dto<'a>(
    tx: &mut Transaction<'_, Postgres>,
    dto: &'a Arbeidssoeker,
) -> anyhow::Result<i64> {
    let rows = arbeidssoeker::select_by_identitetsnummer(
        tx,
        &dto.identitetsnummer,
        0,
        10,
        &SortOrder::Descending,
    )
    .await?;

    let parent_id = if rows.len() > 1 {
        panic!("Fant flere rader for identitetsnummer ({})", rows.len());
    } else if rows.len() == 1 {
        let arbeidssoeker_row = rows.first().unwrap();
        arbeidssoeker_row.id
    } else {
        let arbeidssoeker_row = ArbeidssoekerRow::new(
            dto.aktor_id.clone(),
            dto.arbeidssoeker_id.clone(),
            dto.identitetsnummer.clone(),
            dto.fornavn.clone(),
            dto.mellomnavn.clone(),
            dto.etternavn.clone(),
        );
        arbeidssoeker::insert(tx, &arbeidssoeker_row).await?
    };
    Ok(parent_id)
}
