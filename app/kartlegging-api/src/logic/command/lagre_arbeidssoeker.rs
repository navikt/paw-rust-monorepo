use crate::model::dao::arbeidssoekere::{insert, select_by_identitetsnummer, ArbeidssoekerRow};
use crate::model::dto::bekreftelse::Bekreftelsesloesning;
use crate::model::sort::SortOrder;
use eksterne_hendelser::periode::Periode;
use sqlx::PgPool;

pub async fn lagre_arbeidssoeker(pool: &PgPool, periode: &Periode) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let rows = select_by_identitetsnummer(
        &mut tx,
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
        let row = ArbeidssoekerRow::from_periode(
            -1,
            periode.identitetsnummer.clone(),
            "Kari".to_string(),
            None,
            "Normann".to_string(),
            periode.id,
            periode.startet.tidspunkt,
            periode.avsluttet.as_ref().map(|m| m.tidspunkt),
            vec![Bekreftelsesloesning::Arbeidssoekerregisteret.to_string()],
        );
        insert(&mut tx, &row).await?;
        tracing::debug!("Opprettet innslag for arbeidssøker basert på periode");
    }
    tx.commit().await?;
    Ok(())
}
