use crate::logic::query::{kontortilknytning_query, ledighetsperioder_query};
use crate::model::dao::arbeidssoeker;
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::kontortilknytning::KontorType;
use crate::model::dto::request::{
    IdentitetsnummerQueryRequest, PagingRequest, TilknyttetKontorQueryRequest,
};
use crate::model::dto::response::{KartleggingResponse, PagingResponse};
use crate::model::sort::SortOrder;
use chrono::NaiveDate;
use sqlx::{Postgres, Transaction};

#[tracing::instrument(skip(tx, request))]
pub async fn finn_for_identitetsnummer_query_request(
    tx: &mut Transaction<'_, Postgres>,
    request: &IdentitetsnummerQueryRequest,
) -> anyhow::Result<KartleggingResponse> {
    let identitetsnummer = &request.identitetsnummer;
    let paging = request.paging.clone().unwrap_or_else(|| PagingRequest {
        page: 1,
        page_size: 1000,
        sort_order: SortOrder::Ascending,
    });
    tracing::info!(
        "Finner arbeidssøkere for identitetsnummer, offset {}, limit {}, sort_order {}",
        paging.offset(),
        paging.limit(),
        paging.sort_order.to_string()
    );
    let total_count = arbeidssoeker::count_by_identitetsnummer(tx, &identitetsnummer).await?;
    let arbeidssoeker_rows = arbeidssoeker::select_by_identitetsnummer(
        tx,
        &identitetsnummer,
        paging.offset(),
        paging.limit(),
        &paging.sort_order,
    )
    .await?;
    let arbeidssoekere = map_rows(tx, &paging, &arbeidssoeker_rows).await?;
    let paging_response = PagingResponse {
        page: paging.page,
        page_size: paging.page_size,
        hit_size: arbeidssoekere.len() as i32,
        total_count,
        sort_order: paging.sort_order,
    };
    Ok(KartleggingResponse {
        arbeidssoekere,
        paging: paging_response,
    })
}

#[tracing::instrument(skip(tx, request))]
pub async fn finn_for_kontortilknytning_query_request(
    tx: &mut Transaction<'_, Postgres>,
    request: &TilknyttetKontorQueryRequest,
) -> anyhow::Result<KartleggingResponse> {
    let kontor_id = &request.kontor_id;
    let kontor_typer = request
        .kontor_type
        .clone()
        .map(|kt| vec![kt])
        .unwrap_or(vec![
            KontorType::Arbeidsoppfolging,
            KontorType::Arena,
            KontorType::GeografiskTilknytning,
        ])
        .iter()
        .map(|kt| kt.as_ref().to_string())
        .collect::<Vec<String>>();
    let ledig_siden = request
        .ledig_siden
        .unwrap_or(NaiveDate::from_epoch_days(0).unwrap());
    let paging = request.paging.clone().unwrap_or_else(|| PagingRequest {
        page: 1,
        page_size: 1000,
        sort_order: SortOrder::Ascending,
    });

    let total_count =
        arbeidssoeker::count_by_kontortilknytning(tx, &kontor_id, &kontor_typer, &ledig_siden)
            .await?;
    let kontor_join = kontor_typer
        .iter()
        .map(|k| k.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    tracing::info!(
        "Finner arbeidssøkere for tilknyttet kontor av typer {}, offset {}, limit {}, sort_order {}",
        kontor_join,
        paging.offset(),
        paging.limit(),
        paging.sort_order.to_string()
    );
    let arbeidssoeker_rows = arbeidssoeker::select_by_kontortilknytning(
        tx,
        &kontor_id,
        &kontor_typer,
        &ledig_siden,
        paging.offset(),
        paging.limit(),
        &paging.sort_order,
    )
    .await?;
    let arbeidssoekere = map_rows(tx, &paging, &arbeidssoeker_rows).await?;
    let paging_response = PagingResponse {
        page: paging.page,
        page_size: paging.page_size,
        hit_size: arbeidssoekere.len() as i32,
        total_count,
        sort_order: paging.sort_order,
    };
    Ok(KartleggingResponse {
        arbeidssoekere,
        paging: paging_response,
    })
}

async fn map_rows(
    tx: &mut Transaction<'_, Postgres>,
    paging: &PagingRequest,
    arbeidssoeker_rows: &Vec<ArbeidssoekerRow>,
) -> anyhow::Result<Vec<Arbeidssoeker>> {
    let mut arbeidssoekere = Vec::new();
    for row in arbeidssoeker_rows {
        let ledighetsperioder =
            ledighetsperioder_query::finn_for_parent_id(tx, row.id, paging.clone()).await?;
        let kontortilknytninger =
            kontortilknytning_query::finn_for_aktor_id(tx, &*row.aktor_id).await?;
        arbeidssoekere.push(Arbeidssoeker {
            aktor_id: row.aktor_id.clone(),
            arbeidssoeker_id: row.arbeidssoeker_id.clone(),
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

#[cfg(test)]
mod tests {}
