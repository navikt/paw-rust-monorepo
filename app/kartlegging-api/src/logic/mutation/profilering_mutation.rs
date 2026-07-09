use crate::model::dao::profilering;
use crate::model::dao::profilering::ProfileringRow;
use eksterne_hendelser::profilering::Profilering;
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Profilering,
) -> anyhow::Result<u64> {
    let row = ProfileringRow::new(
        hendelse.id,
        hendelse.periode_id,
        hendelse.opplysninger_om_arbeidssoker_id,
        hendelse.profilert_til.as_ref().to_string(),
        hendelse.sendt_inn_av.tidspunkt,
    );
    let count = profilering::count_by_id(tx, &hendelse.id).await?;
    let rows_affected = if count > 0 {
        profilering::update(tx, &row).await?
    } else {
        profilering::insert(tx, &row).await?
    };
    Ok(rows_affected)
}
