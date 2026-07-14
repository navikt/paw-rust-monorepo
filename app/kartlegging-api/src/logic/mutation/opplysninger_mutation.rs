use crate::model::dao::opplysninger;
use crate::model::dao::opplysninger::OpplysningerRow;
use eksterne_hendelser::opplysninger::Opplysninger;
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Opplysninger,
) -> anyhow::Result<u64> {
    let row = OpplysningerRow::new(
        hendelse.id,
        hendelse.periode_id,
        hendelse
            .jobbsituasjon
            .beskrivelser
            .iter()
            .map(|b| b.beskrivelse.as_ref().to_string())
            .collect(),
        hendelse.sendt_inn_av.tidspunkt,
    );
    let count = opplysninger::count_by_id(tx, &hendelse.id).await?;
    let rows_affected = if count > 1 {
        panic!("Fant flere rader for id ({})", count);
    } else if count == 1 {
        opplysninger::update(tx, &row).await?
    } else {
        opplysninger::insert(tx, &row).await?
    };
    Ok(rows_affected)
}
