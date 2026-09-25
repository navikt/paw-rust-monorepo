use crate::model::dao::ledighetsperiode_kompakt;
use crate::model::dao::ledighetsperiode_kompakt::LedighetsperiodeKompaktRow;
use crate::model::dto::bekreftelse::Bekreftelsesloesning;
use crate::model::dto::ledighetsperiode::LedighetsperiodeKompakt;
use crate::model::dto::profilering::ProfilertTil;
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;
use std::str::FromStr;
use std::vec;

#[tracing::instrument(skip_all)]
pub async fn finn_for_arbeidssoeker_ider(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_ider: &[i64],
) -> anyhow::Result<HashMap<i64, LedighetsperiodeKompakt>> {
    tracing::info!("Henter kartlegging for parent ider");
    let rows =
        ledighetsperiode_kompakt::select_by_arbeidssoeker_ids(tx, arbeidssoeker_ider).await?;

    let mut kartlegginger = HashMap::new();
    for row in &rows {
        kartlegginger.insert(row.arbeidssoeker_id, map_row(row)?);
    }
    Ok(kartlegginger)
}

fn map_row(row: &LedighetsperiodeKompaktRow) -> anyhow::Result<LedighetsperiodeKompakt> {
    let egenvurdert_til = row
        .egenvurdert_til
        .clone()
        .map(|s| ProfilertTil::from_str(&s).unwrap());
    let bekreftelse_paa_vegne_av = if row.bekreftelse_paa_vegne_av.is_empty() {
        // Legge til default på-vegne-av
        vec![Bekreftelsesloesning::Arbeidssoekerregisteret]
    } else {
        row.bekreftelse_paa_vegne_av
            .iter()
            .map(|s| Bekreftelsesloesning::from_str(s).unwrap())
            .collect()
    };

    Ok(LedighetsperiodeKompakt {
        periode_id: row.periode_id,
        ledig_siden: row.arbeidsledig_fra,
        periode_startet: row.arbeidssoeker_fra,
        periode_avsluttet: row.arbeidssoeker_til,
        egenvurdert_til,
        bekreftelse_har_jobbet: row.bekreftelse_har_jobbet,
        bekreftelse_vil_fortsette: row.bekreftelse_vil_fortsette,
        bekreftelse_paa_vegne_av,
    })
}
