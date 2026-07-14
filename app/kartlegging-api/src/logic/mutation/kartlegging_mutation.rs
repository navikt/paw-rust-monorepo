use crate::model::dao::kartlegging;
use crate::model::dao::kartlegging::KartleggingRow;
use eksterne_hendelser::periode::Periode;
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: i64,
    hendelse: &'a Periode,
) -> anyhow::Result<()> {
    let kartlegginger = kartlegging::select_by_periode_id(tx, &hendelse.id).await?;
    if kartlegginger.len() > 1 {
        panic!("Fant flere rader for periode-id ({})", kartlegginger.len());
    } else if kartlegginger.len() == 1 {
        let kartlegging = kartlegginger.first().unwrap();
    } else {
        let kartlegging_row = KartleggingRow::new(
            hendelse.id.clone(),
            parent_id,
            hendelse.startet.tidspunkt.clone(),
            None,
        );
        kartlegging::insert(tx, &kartlegging_row).await?;
    }
    Ok(())
}
