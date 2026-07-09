use crate::model::dao::arbeidssoeker;
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::sort::SortOrder;
use sqlx::{Postgres, Transaction};

pub async fn lagre_dto<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Arbeidssoeker,
) -> anyhow::Result<i64> {
    let rows = arbeidssoeker::select_by_identitetsnummer(
        tx,
        &hendelse.identitetsnummer,
        0,
        10,
        &SortOrder::Descending,
    )
    .await?;

    let parent_id = if rows.len() > 1 {
        panic!("Fant flere rader for samme arbeidssøker ({})", rows.len());
    } else if rows.len() == 1 {
        let arbeidssoeker_row = rows.first().unwrap();
        arbeidssoeker_row.id
    } else {
        let arbeidssoeker_row = ArbeidssoekerRow::new(
            hendelse.arbeidssoeker_id,
            hendelse.identitetsnummer.clone(),
            hendelse.fornavn.clone(),
            hendelse.mellomnavn.clone(),
            hendelse.etternavn.clone(),
        );
        arbeidssoeker::insert(tx, &arbeidssoeker_row).await?
    };
    Ok(parent_id)
}
