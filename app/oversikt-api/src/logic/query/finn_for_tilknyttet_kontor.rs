use crate::model::dao::{arbeidssoekere, tilknyttet_kontor};
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::bekreftelse::{Bekreftelse, Bekreftelsesloesning};
use crate::model::dto::egenvurdering::Egenvurdering;
use crate::model::dto::kontor::TilknyttetKontor;
use crate::model::dto::opplysninger::Opplysninger;
use crate::model::dto::periode::Periode;
use crate::model::dto::profilering::{Profilering, ProfilertTil};
use crate::model::dto::request::{PagingRequest, TilknyttetKontorQueryRequest};
use crate::model::dto::response::{OversiktResponse, PagingResponse};
use crate::model::sort::SortOrder;
use sqlx::PgPool;
use std::str::FromStr;

pub async fn finn_for_tilknyttet_kontor(
    pool: &PgPool,
    request: &TilknyttetKontorQueryRequest,
) -> anyhow::Result<OversiktResponse> {
    let paging = request.paging.clone().unwrap_or_else(|| PagingRequest {
        page: 1,
        page_size: 1000,
        sort_order: SortOrder::Ascending,
    });
    let mut tx = pool.begin().await?;
    tracing::info!("Henter total antall arbeidssøkere for identitetsnummer");
    let total_items =
        arbeidssoekere::count_by_identitetsnummer(&mut tx, &request.identitetsnummer).await?;
    tracing::info!(
        "Henter arbeidssøkere for identitetsnummer, offset {}, limit {}, sort_order {}",
        paging.offset(),
        paging.limit(),
        paging.sort_order.to_string()
    );
    let arbeidssoeker_rows = arbeidssoekere::select_by_identitetsnummer(
        &mut tx,
        &request.identitetsnummer,
        paging.offset(),
        paging.limit(),
        &paging.sort_order,
    )
    .await?;
    let mut arbeidssoekere = Vec::new();
    for row in &arbeidssoeker_rows {
        tracing::info!("Henter tilknyttede kontorer for parent id");
        let tilknyttet_kontor_rows =
            tilknyttet_kontor::select_by_parent_id(&mut tx, &row.id).await?;
        let tilknyttet_kontor = tilknyttet_kontor_rows
            .into_iter()
            .map(|kontor_row| TilknyttetKontor {
                kontor_id: kontor_row.kontor_id,
                kontor_navn: kontor_row.kontor_navn,
                kontor_type: kontor_row.kontor_type,
            })
            .collect();
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
    tx.commit().await?;
    let paging_response = PagingResponse {
        page: paging.page,
        page_size: paging.page_size,
        total_items,
        sort_order: paging.sort_order,
    };
    Ok(OversiktResponse {
        arbeidssoekere,
        paging: paging_response,
    })
}
