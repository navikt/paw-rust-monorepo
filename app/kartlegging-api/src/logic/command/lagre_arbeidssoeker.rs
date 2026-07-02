use crate::model::dao::arbeidssoekere;
use crate::model::dao::arbeidssoekere::ArbeidssoekerRow;
use crate::model::dao::ledighetsperioder::LedighetsperiodeRow;
use crate::model::sort::SortOrder;
use eksterne_hendelser::periode::Periode;
use sqlx::{PgPool, Postgres, Transaction};

pub async fn lagre_arbeidssoeker<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode: &'a Periode,
) -> anyhow::Result<i64> {
    let rows = arbeidssoekere::select_by_identitetsnummer(
        tx,
        &periode.identitetsnummer,
        0,
        10,
        &SortOrder::Descending,
    )
    .await?;
    if rows.len() > 1 {
        panic!("Fant flere rader for samme arbeidssøker ({})", rows.len());
    } else if rows.len() == 1 {
        let row = rows.first().unwrap();
    } else {
        let arbeidssoeker_row = ArbeidssoekerRow::new(
            -1,
            periode.identitetsnummer.clone(),
            "Kari".to_string(),
            None,
            "Normann".to_string(),
        );
        let id = arbeidssoekere::insert(tx, &arbeidssoeker_row).await?;
        let ledighetsperiode_row = LedighetsperiodeRow::new(
            id,
            periode.id,
            periode.startet.tidspunkt,
            periode.avsluttet.as_ref().map(|m| m.tidspunkt),
        );
        tracing::debug!("Opprettet innslag for arbeidssøker basert på periode");
    }
    Ok(-1)
}
