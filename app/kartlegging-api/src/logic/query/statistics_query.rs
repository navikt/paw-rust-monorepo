use crate::model::dao::kartlegging;
use crate::model::dto::response::StatisticsResponse;
use sqlx::{Postgres, Transaction};

#[tracing::instrument(skip_all)]
pub async fn finn(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<StatisticsResponse> {
    tracing::info!("Finner statistikk for arbeidssøkere",);
    let rows = kartlegging::count_metrics(tx).await?;
    Ok(StatisticsResponse {
        total: rows.total,
        is_null: rows.is_null,
        is_not_null: rows.is_not_null,
        over_0030_days: rows.over_0030_days,
        over_0060_days: rows.over_0060_days,
        over_0090_days: rows.over_0090_days,
        over_0180_days: rows.over_0180_days,
        over_0365_days: rows.over_0365_days,
        over_0730_days: rows.over_0730_days,
        over_1095_days: rows.over_1095_days,
    })
}

#[cfg(test)]
mod tests {}
