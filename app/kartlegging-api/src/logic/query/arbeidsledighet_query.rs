use crate::logic::query::{kontortilknytning_query, ledighetsperioder_kompakt_query};
use crate::model::dao::arbeidssoeker;
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::dto::arbeidssoeker::ArbeidssoekerKompakt;
use crate::model::dto::kontortilknytning::KontorType;
use crate::model::dto::request::{
    IdentitetsnummerQueryRequest, PagingRequest, TilknyttetKontorQueryRequest,
};
use crate::model::dto::response::ArbeidsledighetResponse;
use crate::model::sort::SortOrder;
use chrono::NaiveDate;
use sqlx::{Postgres, Transaction};

#[tracing::instrument(skip_all)]
pub async fn finn_for_identitetsnummer_query_request(
    tx: &mut Transaction<'_, Postgres>,
    request: &IdentitetsnummerQueryRequest,
) -> anyhow::Result<ArbeidsledighetResponse> {
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
    let arbeidssoeker_rows =
        arbeidssoeker::select_by_identitetsnummer(tx, &identitetsnummer).await?;
    let arbeidssoekere = map_rows(tx, &arbeidssoeker_rows).await?;
    let hit_size = arbeidssoekere.len() as i32;
    let total_count = arbeidssoeker_rows.len() as i64;
    Ok(ArbeidsledighetResponse {
        arbeidssoekere,
        paging: paging.as_response(hit_size, total_count),
    })
}

#[tracing::instrument(skip_all)]
pub async fn finn_for_kontortilknytning_query_request(
    tx: &mut Transaction<'_, Postgres>,
    request: &TilknyttetKontorQueryRequest,
) -> anyhow::Result<ArbeidsledighetResponse> {
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
    let arbeidssoekere = map_rows(tx, &arbeidssoeker_rows).await?;
    let hit_size = arbeidssoekere.len() as i32;
    Ok(ArbeidsledighetResponse {
        arbeidssoekere,
        paging: paging.as_response(hit_size, total_count),
    })
}

async fn map_rows(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_rows: &Vec<ArbeidssoekerRow>,
) -> anyhow::Result<Vec<ArbeidssoekerKompakt>> {
    let arbeidssoeker_ider: Vec<i64> = arbeidssoeker_rows.iter().map(|row| row.id).collect();
    let aktor_ider: Vec<String> = arbeidssoeker_rows
        .iter()
        .map(|row| row.aktor_id.clone())
        .collect();

    let mut ledighetsperioder_by_arbeidssoeker_id =
        ledighetsperioder_kompakt_query::finn_for_arbeidssoeker_ider(tx, &arbeidssoeker_ider).await?;
    let mut kontortilknytninger_by_aktor_id =
        kontortilknytning_query::finn_for_aktor_ider(tx, &aktor_ider).await?;

    let mut arbeidssoekere = Vec::new();
    for row in arbeidssoeker_rows {
        let ledighetsperioder = ledighetsperioder_by_arbeidssoeker_id
            .remove(&row.id)
            .into_iter()
            .collect();
        let kontortilknytninger = kontortilknytninger_by_aktor_id
            .remove(&row.aktor_id)
            .unwrap_or_default();
        arbeidssoekere.push(ArbeidssoekerKompakt::new(
            row.id.clone(),
            row.aktor_id.clone(),
            row.identitetsnummer.clone(),
            row.fornavn.clone(),
            row.mellomnavn.clone(),
            row.etternavn.clone(),
            ledighetsperioder,
            kontortilknytninger,
        ))
    }
    Ok(arbeidssoekere)
}
