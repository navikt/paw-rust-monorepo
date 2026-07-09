use crate::model::dao::periode;
use crate::model::dao::periode::PeriodeRow;
use eksterne_hendelser::periode::Periode;
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Periode,
) -> anyhow::Result<u64> {
    let row = PeriodeRow::new(
        hendelse.id,
        hendelse.identitetsnummer.clone(),
        hendelse.startet.tidspunkt,
        hendelse.avsluttet.as_ref().map(|m| m.tidspunkt),
    );
    let count = periode::count_by_id(tx, &hendelse.id).await?;
    let rows_affected = if count > 0 {
        periode::update(tx, &row).await?
    } else {
        periode::insert(tx, &row).await?
    };
    Ok(rows_affected)
}
