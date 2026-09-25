use crate::model::dao::ledighetsperiode;
use crate::model::dao::ledighetsperiode::LedighetsperiodeRow;
use crate::model::dto::bekreftelse::{Bekreftelse, Bekreftelsesloesning};
use crate::model::dto::egenvurdering::Egenvurdering;
use crate::model::dto::ledighetsperiode::Ledighetsperiode;
use crate::model::dto::opplysninger::{Jobbsituasjon, Opplysninger};
use crate::model::dto::periode::Periode;
use crate::model::dto::profilering::{Profilering, ProfilertTil};
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;
use std::str::FromStr;

#[tracing::instrument(skip_all)]
pub async fn finn_for_arbeidssoeker_ider(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_ider: &[i64],
) -> anyhow::Result<HashMap<i64, Ledighetsperiode>> {
    tracing::info!("Henter kartlegging for parent ider");
    let rows = ledighetsperiode::select_by_arbeidssoeker_ids(tx, arbeidssoeker_ider).await?;

    let mut kartlegginger = HashMap::new();
    for row in &rows {
        kartlegginger.insert(row.arbeidssoeker_id, map_row(row)?);
    }
    Ok(kartlegginger)
}

fn map_row(row: &LedighetsperiodeRow) -> anyhow::Result<Ledighetsperiode> {
    let periode = Some(Periode {
        id: row.periode_id,
        startet: row.periode_startet,
        avsluttet: row.periode_avsluttet,
    });
    let opplysninger = match (row.opplysninger_id, row.opplysninger_tidspunkt) {
        (Some(id), Some(tidspunkt)) => {
            let mut jobbsituasjon = Vec::new();
            for j in &row.opplysninger_jobbsituasjon {
                jobbsituasjon.push(Jobbsituasjon::from_str(j.as_str())?);
            }
            Some(Opplysninger {
                id,
                jobbsituasjon,
                tidspunkt,
            })
        }
        _ => None,
    };
    let profilering = match (
        row.profilering_id,
        row.profilert_til.clone(),
        row.profilering_tidspunkt,
    ) {
        (Some(id), Some(profilert_til), Some(tidspunkt)) => Some(Profilering {
            id,
            profilert_til: ProfilertTil::from_str(profilert_til.as_str())?,
            tidspunkt,
        }),
        _ => None,
    };
    let egenvurdering = match (
        row.egenvurdering_id,
        row.egenvurdert_til.clone(),
        row.egenvurdering_tidspunkt,
    ) {
        (Some(id), Some(egenvurdert_til), Some(tidspunkt)) => Some(Egenvurdering {
            id,
            egenvurdert_til: ProfilertTil::from_str(egenvurdert_til.as_str())?,
            tidspunkt,
        }),
        _ => None,
    };
    let bekreftelse = match (
        row.bekreftelse_id,
        row.bekreftelse_gjelder_fra,
        row.bekreftelse_gjelder_til,
        row.bekreftelse_har_jobbet,
        row.bekreftelse_vil_fortsette,
        row.bekreftelsesloesning.clone(),
    ) {
        (
            Some(id),
            Some(gjelder_fra),
            Some(gjelder_til),
            Some(har_jobbet),
            Some(vil_fortsette),
            Some(bekreftelsesloesning),
        ) => Some(Bekreftelse {
            id,
            gjelder_fra,
            gjelder_til,
            har_jobbet,
            vil_fortsette,
            bekreftelsesloesning: Bekreftelsesloesning::from_str(bekreftelsesloesning.as_str())?,
        }),
        _ => None,
    };
    let bekreftelse_paa_vegne_av = if row.bekreftelse_paa_vegne_av.is_empty() {
        // Legge til default på-vegne-av
        vec![Bekreftelsesloesning::Arbeidssoekerregisteret]
    } else {
        row.bekreftelse_paa_vegne_av
            .iter()
            .map(|s| Bekreftelsesloesning::from_str(s).unwrap())
            .collect()
    };

    Ok(Ledighetsperiode {
        ledig_siden: row.arbeidsledig_fra,
        periode,
        opplysninger,
        profilering,
        egenvurdering,
        bekreftelse,
        bekreftelse_paa_vegne_av,
    })
}
