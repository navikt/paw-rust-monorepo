use crate::logic::query::mapper_v2;
use crate::model::dao::arbeidssoekere_v2;
use crate::model::dto::request::{IdentitetsnummerQueryRequest, PagingRequest};
use crate::model::dto::response::PagingResponse;
use crate::model::dto::response_v2::KartleggingResponse;
use crate::model::sort::SortOrder;
use sqlx::{PgPool, Postgres, Transaction};

#[tracing::instrument(skip(tx))]
pub async fn finn_for_identitetsnummer_v2(
    tx: &mut Transaction<'_, Postgres>,
    request: &IdentitetsnummerQueryRequest,
) -> anyhow::Result<KartleggingResponse> {
    let paging = request.paging.clone().unwrap_or_else(|| PagingRequest {
        page: 1,
        page_size: 1000,
        sort_order: SortOrder::Ascending,
    });
    let total_count =
        arbeidssoekere_v2::count_by_identitetsnummer(tx, &request.identitetsnummer).await?;
    tracing::info!(
        "Finner arbeidssøkere for identitetsnummer, offset {}, limit {}, sort_order {}",
        paging.offset(),
        paging.limit(),
        paging.sort_order.to_string()
    );
    let arbeidssoeker_rows = arbeidssoekere_v2::select_by_identitetsnummer(
        tx,
        &request.identitetsnummer,
        paging.offset(),
        paging.limit(),
        &paging.sort_order,
    )
    .await?;
    let arbeidssoekere = mapper_v2::map_rows(tx, &paging, &arbeidssoeker_rows).await?;
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
