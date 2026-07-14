use crate::model::dao::bekreftelse;
use crate::model::dao::bekreftelse::BekreftelseRow;
use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Bekreftelse,
) -> anyhow::Result<u64> {
    let row = BekreftelseRow::new(
        hendelse.id,
        hendelse.periode_id,
        hendelse.svar.gjelder_fra,
        hendelse.svar.gjelder_til,
        hendelse.svar.har_jobbet_i_denne_perioden,
        hendelse.svar.vil_fortsette_som_arbeidssoeker,
        hendelse.bekreftelsesloesning.as_ref().to_string(),
        hendelse.svar.sendt_inn_av.tidspunkt,
    );
    let count = bekreftelse::count_by_id(tx, &hendelse.id).await?;
    let rows_affected = if count > 1 {
        panic!("Fant flere rader for id ({})", count);
    } else if count == 1 {
        bekreftelse::update(tx, &row).await?
    } else {
        bekreftelse::insert(tx, &row).await?
    };
    Ok(rows_affected)
}
