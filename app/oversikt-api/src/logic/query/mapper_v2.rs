use crate::model::dao::arbeidssoekere::ArbeidssoekerRow;
use crate::model::dao::arbeidssoekere_v2::ArbeidssoekerRowV2;
use crate::model::dao::{ledighetsperioder, tilknyttet_kontor};
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::arbeidssoeker_v2::ArbeidssoekerV2;
use crate::model::dto::bekreftelse::{Bekreftelse, Bekreftelsesloesning};
use crate::model::dto::egenvurdering::Egenvurdering;
use crate::model::dto::kontor::{KontorType, Kontortilknytning, TilknyttetKontor};
use crate::model::dto::ledighetsperiode::Ledighetsperiode;
use crate::model::dto::opplysninger::Opplysninger;
use crate::model::dto::periode::Periode;
use crate::model::dto::profilering::{Profilering, ProfilertTil};
use crate::model::dto::request::PagingRequest;
use sqlx::{Postgres, Transaction};
use std::str::FromStr;

pub async fn map_rows(
    tx: &mut Transaction<'_, Postgres>,
    paging: &PagingRequest,
    arbeidssoeker_rows: &Vec<ArbeidssoekerRowV2>,
) -> anyhow::Result<Vec<ArbeidssoekerV2>> {
    let mut arbeidssoekere = Vec::new();
    for row in arbeidssoeker_rows {
        tracing::info!("Henter tilknyttede kontorer for parent id");
        let tilknyttet_kontor_rows = tilknyttet_kontor::select_by_parent_id(tx, &row.id).await?;
        let mut kontortilknytninger = Vec::new();
        for kontor_row in &tilknyttet_kontor_rows {
            kontortilknytninger.push(Kontortilknytning {
                kontor_id: kontor_row.kontor_id.clone(),
                kontor_navn: kontor_row.kontor_navn.clone(),
                kontor_type: KontorType::from_str(kontor_row.kontor_type.as_str())?,
            });
        }

        tracing::info!("Henter ledighetsperioder for parent id");
        let ledighetsperiode_rows = ledighetsperioder::select_by_parent_id(
            tx,
            row.id,
            paging.offset(),
            paging.limit(),
            &paging.sort_order,
        )
        .await?;
        let mut ledighetsperioder = Vec::new();
        for ledighetsperiode_row in &ledighetsperiode_rows {
            let periode = Periode {
                id: ledighetsperiode_row.periode_id,
                startet: ledighetsperiode_row.periode_startet,
                avsluttet: ledighetsperiode_row.periode_avsluttet,
            };
            let opplysninger = match (
                ledighetsperiode_row.opplysninger_id,
                ledighetsperiode_row.opplysninger_tidspunkt,
            ) {
                (Some(id), Some(tidspunkt)) => Some(Opplysninger { id, tidspunkt }),
                _ => None,
            };
            let profilering = match (
                ledighetsperiode_row.profilering_id,
                ledighetsperiode_row.profilert_til.clone(),
                ledighetsperiode_row.profilering_tidspunkt,
            ) {
                (Some(id), Some(profilert_til), Some(tidspunkt)) => Some(Profilering {
                    id,
                    profilert_til: ProfilertTil::from_str(profilert_til.as_str())?,
                    tidspunkt,
                }),
                _ => None,
            };
            let egenvurdering = match (
                ledighetsperiode_row.egenvurdering_id,
                ledighetsperiode_row.egenvurdert_til.clone(),
                ledighetsperiode_row.egenvurdering_tidspunkt,
            ) {
                (Some(id), Some(egenvurdert_til), Some(tidspunkt)) => Some(Egenvurdering {
                    id,
                    egenvurdert_til: ProfilertTil::from_str(egenvurdert_til.as_str())?,
                    tidspunkt,
                }),
                _ => None,
            };
            let bekreftelse = match (
                ledighetsperiode_row.bekreftelse_id,
                ledighetsperiode_row.bekreftelse_gjelder_fra,
                ledighetsperiode_row.bekreftelse_gjelder_til,
                ledighetsperiode_row.bekreftelse_har_jobbet,
                ledighetsperiode_row.bekreftelse_vil_fortsette,
                ledighetsperiode_row.bekreftelsesloesning.clone(),
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
            for loesning in &ledighetsperiode_row.bekreftelse_paa_vegne_av {
                bekreftelse_paa_vegne_av.push(Bekreftelsesloesning::from_str(loesning.as_str())?);
            }
            let bekreftelse_paa_vegne_av = bekreftelse_paa_vegne_av; // Fjern mut ref

            ledighetsperioder.push(Ledighetsperiode {
                ledig_siden: ledighetsperiode_row.ledig_siden,
                periode,
                opplysninger,
                profilering,
                egenvurdering,
                bekreftelse,
                bekreftelse_paa_vegne_av,
            });
        }

        arbeidssoekere.push(ArbeidssoekerV2 {
            arbeidssoeker_id: row.arbeidssoeker_id,
            identitetsnummer: row.identitetsnummer.clone(),
            fornavn: row.fornavn.clone(),
            mellomnavn: row.mellomnavn.clone(),
            etternavn: row.etternavn.clone(),
            ledighetsperioder,
            kontortilknytninger,
        })
    }
    Ok(arbeidssoekere)
}
