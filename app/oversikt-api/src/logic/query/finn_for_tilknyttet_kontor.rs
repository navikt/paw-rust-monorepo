use crate::logic::query::mapper;
use crate::model::dao::arbeidssoekere;
use crate::model::dto::kontor::KontorType;
use crate::model::dto::request::{PagingRequest, TilknyttetKontorQueryRequest};
use crate::model::dto::response::{OversiktResponse, PagingResponse};
use crate::model::sort::SortOrder;
use chrono::DateTime;
use sqlx::PgPool;

pub async fn finn_for_tilknyttet_kontor(
    pool: &PgPool,
    request: &TilknyttetKontorQueryRequest,
) -> anyhow::Result<OversiktResponse> {
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
    let ledig_side = request.ledig_siden.unwrap_or(DateTime::UNIX_EPOCH.into());
    let paging = request.paging.clone().unwrap_or_else(|| PagingRequest {
        page: 1,
        page_size: 1000,
        sort_order: SortOrder::Ascending,
    });

    let mut tx = pool.begin().await?;
    tracing::info!("Henter total antall arbeidssøkere for tilknyttet kontor");
    let total_count =
        arbeidssoekere::count_by_tilknyttet_kontor(&mut tx, &kontor_id, &kontor_typer, &ledig_side)
            .await?;
    tracing::info!(
        "Henter arbeidssøkere for tilknyttet kontor av typer {}, offset {}, limit {}, sort_order {}",
        String::from_iter(kontor_typer.clone()),
        paging.offset(),
        paging.limit(),
        paging.sort_order.to_string()
    );
    let arbeidssoeker_rows = arbeidssoekere::select_by_tilknyttet_kontor(
        &mut tx,
        &kontor_id,
        &kontor_typer,
        &ledig_side,
        paging.offset(),
        paging.limit(),
        &paging.sort_order,
    )
    .await?;
    let arbeidssoekere = mapper::map_rows(&mut tx, &arbeidssoeker_rows).await?;
    tx.commit().await?;
    let paging_response = PagingResponse {
        page: paging.page,
        page_size: paging.page_size,
        hit_size: arbeidssoekere.len() as i32,
        total_count,
        sort_order: paging.sort_order,
    };
    Ok(OversiktResponse {
        arbeidssoekere,
        paging: paging_response,
    })
}

#[cfg(test)]
mod tests {}
