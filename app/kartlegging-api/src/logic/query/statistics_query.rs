use crate::model::dao::kartlegging;
use crate::model::dto::response::StatisticsResponse;
use sqlx::{Postgres, Transaction};

#[tracing::instrument(skip(tx))]
pub async fn finn(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<StatisticsResponse> {
    tracing::info!("Finner statistikk for arbeidssøkere",);
    let rows = kartlegging::count_metrics(tx).await?;
    Ok(StatisticsResponse {
        total: rows.total,
        is_null: rows.is_null,
        is_not_null: rows.is_not_null,
        over_30_days: rows.over_30_days,
        over_60_days: rows.over_60_days,
        over_90_days: rows.over_90_days,
        over_180_days: rows.over_180_days,
        over_365_days: rows.over_365_days,
    })
}

#[cfg(test)]
mod tests {}
