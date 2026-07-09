use crate::model::dao::bekreftelse_paavegneav;
use crate::model::dao::bekreftelse_paavegneav::BekreftelsePaaVegneAvRow;
use eksterne_hendelser::bekreftelse::paa_vegne_av::{Handling, PaaVegneAv};
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a PaaVegneAv,
) -> anyhow::Result<u64> {
    let handling = &hendelse.handling;
    let periode_id = hendelse.periode_id;
    let bekreftelsesloesning = hendelse.bekreftelsesloesning.as_ref().to_string();
    let rows = bekreftelse_paavegneav::select_by_periode_id(tx, &periode_id).await?;

    if rows.len() > 1 {
        panic!("Fant flere rader for samme periode ({})", rows.len());
    } else if rows.len() == 1 {
        let row = rows.first().unwrap();
        return match handling {
            Handling::Start(_) => {
                let mut bekreftelsesloesninger = row.bekreftelsesloesninger.clone();
                bekreftelsesloesninger.push(bekreftelsesloesning);
                bekreftelsesloesninger.sort_unstable();
                bekreftelsesloesninger.dedup();
                let bekreftelsesloesninger = bekreftelsesloesninger;
                let updated_row = BekreftelsePaaVegneAvRow::new(periode_id, bekreftelsesloesninger);
                bekreftelse_paavegneav::update(tx, &updated_row).await
            }
            Handling::Stopp(_) => {
                let bekreftelsesloesninger = row
                    .bekreftelsesloesninger
                    .iter()
                    .filter(|&l| l != &bekreftelsesloesning)
                    .map(|l| l.to_string())
                    .collect();
                let updated_row = BekreftelsePaaVegneAvRow::new(periode_id, bekreftelsesloesninger);
                bekreftelse_paavegneav::update(tx, &updated_row).await
            }
        };
    } else {
        return match handling {
            Handling::Start(_) => {
                let bekreftelsesloesninger = vec![bekreftelsesloesning];
                let row = BekreftelsePaaVegneAvRow::new(periode_id, bekreftelsesloesninger);
                bekreftelse_paavegneav::insert(tx, &row).await
            }
            Handling::Stopp(_) => {
                tracing::warn!("Mottok stopp for på-vegne-av som ikke finnes i databasen");
                Ok(0u64)
            }
        };
    }
}
