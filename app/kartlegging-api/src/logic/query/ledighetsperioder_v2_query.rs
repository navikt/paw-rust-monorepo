use crate::model::dao::ledighetsperiode_v2;
use crate::model::dao::ledighetsperiode_v2::LedighetsperiodeV2Row;
use crate::model::dto::bekreftelse::Bekreftelsesloesning;
use crate::model::dto::ledighetsperiode_v2::LedighetsperiodeV2;
use crate::model::dto::profilering::ProfilertTil;
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;
use std::str::FromStr;

#[tracing::instrument(skip_all)]
pub async fn finn_for_arbeidssoeker_ider(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_ider: &[i64],
) -> anyhow::Result<HashMap<i64, LedighetsperiodeV2>> {
    tracing::info!("Henter kartlegging for parent ider");
    let rows = ledighetsperiode_v2::select_by_arbeidssoeker_ids(tx, arbeidssoeker_ider).await?;

    let mut kartlegginger = HashMap::new();
    for row in &rows {
        kartlegginger.insert(row.arbeidssoeker_id, map_row(row)?);
    }
    Ok(kartlegginger)
}

fn map_row(row: &LedighetsperiodeV2Row) -> anyhow::Result<LedighetsperiodeV2> {
    let egenvurdert_til = row
        .egenvurdert_til
        .clone()
        .map(|s| ProfilertTil::from_str(&s).unwrap());
    let bekreftelse_ansvar = if row.bekreftelse_ansvar.is_empty() {
        // Legge til default på-vegne-av
        Bekreftelsesloesning::Arbeidssoekerregisteret
    } else {
        let loesning = row.bekreftelse_ansvar.first().unwrap();
        Bekreftelsesloesning::from_str(loesning)?
    };

    Ok(LedighetsperiodeV2 {
        periode_id: row.periode_id,
        ledig_siden: row.arbeidsledig_fra,
        periode_startet: row.arbeidssoeker_fra,
        periode_avsluttet: row.arbeidssoeker_til,
        egenvurdert_til,
        bekreftelse_har_jobbet: row.bekreftelse_har_jobbet,
        bekreftelse_vil_fortsette: row.bekreftelse_vil_fortsette,
        bekreftelse_ansvar,
    })
}
