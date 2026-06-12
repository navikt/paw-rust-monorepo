use crate::model::dao::arbeidssoekere::ArbeidssoekerRow;
use crate::model::dao::{arbeidssoekere, tilknyttet_kontor};
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::bekreftelse::{Bekreftelse, Bekreftelsesloesning};
use crate::model::dto::egenvurdering::Egenvurdering;
use crate::model::dto::kontor::{KontorType, TilknyttetKontor};
use crate::model::dto::opplysninger::Opplysninger;
use crate::model::dto::periode::Periode;
use crate::model::dto::profilering::{Profilering, ProfilertTil};
use sqlx::{Postgres, Transaction};
use std::str::FromStr;

pub async fn map_rows(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_rows: &Vec<ArbeidssoekerRow>,
) -> anyhow::Result<Vec<Arbeidssoeker>> {
    let mut arbeidssoekere = Vec::new();
    for row in arbeidssoeker_rows {
        tracing::info!("Henter tilknyttede kontorer for parent id");
        let tilknyttet_kontor_rows = tilknyttet_kontor::select_by_parent_id(tx, &row.id).await?;
        let mut tilknyttet_kontor = Vec::new();
        for kontor_row in &tilknyttet_kontor_rows {
            tilknyttet_kontor.push(TilknyttetKontor {
                kontor_id: kontor_row.kontor_id.clone(),
                kontor_navn: kontor_row.kontor_navn.clone(),
                kontor_type: KontorType::from_str(kontor_row.kontor_type.as_str())?,
            });
        }
        let periode = Periode {
            id: row.periode_id,
            startet: row.periode_startet,
            avsluttet: row.periode_avsluttet,
        };
        let opplysninger = match (row.opplysninger_id, row.opplysninger_tidspunkt) {
            (Some(id), Some(tidspunkt)) => Some(Opplysninger { id, tidspunkt }),
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
                bekreftelsesloesning: Bekreftelsesloesning::from_str(
                    bekreftelsesloesning.as_str(),
                )?,
            }),
            _ => None,
        };
        let mut bekreftelse_paa_vegne_av = Vec::new();
        for loesning in &row.bekreftelse_paa_vegne_av {
            bekreftelse_paa_vegne_av.push(Bekreftelsesloesning::from_str(loesning.as_str())?);
        }
        let bekreftelse_paa_vegne_av = bekreftelse_paa_vegne_av; // Fjern mut ref

        arbeidssoekere.push(Arbeidssoeker {
            arbeidssoeker_id: row.arbeidssoeker_id,
            identitetsnummer: row.identitetsnummer.clone(),
            fornavn: row.fornavn.clone(),
            mellomnavn: row.mellomnavn.clone(),
            etternavn: row.etternavn.clone(),
            ledig_siden: row.ledig_siden,
            periode,
            opplysninger,
            profilering,
            egenvurdering,
            bekreftelse,
            bekreftelse_paa_vegne_av,
            tilknyttet_kontor,
        })
    }
    Ok(arbeidssoekere)
}
