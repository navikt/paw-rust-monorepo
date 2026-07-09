use crate::model::dao::egenvurdering::EgenvurderingRow;
use crate::model::dao::egenvurdering;
use eksterne_hendelser::egenvurdering::Egenvurdering;
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Egenvurdering,
) -> anyhow::Result<u64> {
    let row = EgenvurderingRow::new(
        hendelse.id,
        hendelse.periode_id,
        hendelse.profilering_id,
        hendelse.profilert_til.as_ref().to_string(),
        hendelse.egenvurdering.as_ref().to_string(),
        hendelse.sendt_inn_av.tidspunkt,
    );
    let count = egenvurdering::count_by_id(tx, &hendelse.id).await?;
    let rows_affected = if count > 0 {
        egenvurdering::update(tx, &row).await?
    } else {
        egenvurdering::insert(tx, &row).await?
    };
    Ok(rows_affected)
}
