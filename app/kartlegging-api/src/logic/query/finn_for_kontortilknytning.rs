use crate::logic::query::mapper;
use crate::model::dao::arbeidssoekere;
use crate::model::dto::kontor::KontorType;
use crate::model::dto::request::{PagingRequest, TilknyttetKontorQueryRequest};
use crate::model::dto::response::{KartleggingResponse, PagingResponse};
use crate::model::sort::SortOrder;
use chrono::NaiveDate;
use sqlx::{Postgres, Transaction};

#[tracing::instrument(skip(tx))]
pub async fn finn_for_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    request: &TilknyttetKontorQueryRequest,
) -> anyhow::Result<KartleggingResponse> {
    let kontor_id = request.kontor_id.clone();
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
    let ledig_side = request
        .ledig_siden
        .unwrap_or(NaiveDate::from_epoch_days(0).unwrap());
    let paging = request.paging.clone().unwrap_or_else(|| PagingRequest {
        page: 1,
        page_size: 1000,
        sort_order: SortOrder::Ascending,
    });

    let total_count =
        arbeidssoekere::count_by_kontortilknytning(tx, &kontor_id, &kontor_typer, &ledig_side)
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
    let arbeidssoeker_rows = arbeidssoekere::select_by_kontortilknytning(
        tx,
        &kontor_id,
        &kontor_typer,
        &ledig_side,
        paging.offset(),
        paging.limit(),
        &paging.sort_order,
    )
    .await?;
    let arbeidssoekere = mapper::map_rows(tx, &paging, &arbeidssoeker_rows).await?;
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

#[cfg(test)]
mod tests {}
