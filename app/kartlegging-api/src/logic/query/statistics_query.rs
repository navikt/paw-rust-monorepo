use crate::model::dao::statistics;
use crate::model::dao::statistics::KartleggingStatisticsRow;
use crate::model::dto::response::StatisticsResponse;
use crate::model::dto::statistics::{LedighetStatistics, PeriodeStatistics};
use sqlx::{Postgres, Transaction};

#[tracing::instrument(skip_all)]
pub async fn finn(
    tx: &mut Transaction<'_, Postgres>,
    optional_kontor_id: Option<String>,
) -> anyhow::Result<StatisticsResponse> {
    tracing::info!("Finner statistikk for arbeidssøkere");
    let row = match optional_kontor_id {
        None => statistics::count(tx).await?,
        Some(kontor_id) => {
            let kontor_id_list = vec![kontor_id];
            statistics::count_by_kontor_id(tx, &kontor_id_list).await?
        }
    };
    let response = map_row(row);
    Ok(response)
}

fn map_row(row: KartleggingStatisticsRow) -> StatisticsResponse {
    StatisticsResponse {
        perioder: PeriodeStatistics {
            totalt: row.perioder_total,
            er_aktiv: row.perioder_aktiv,
            er_avsluttet: row.perioder_avsluttet,
        },
        ledighet: LedighetStatistics {
            er_null: row.ledighet_null,
            er_ikke_null: row.ledighet_ikke_null,
            over_0030_dager: row.ledighet_over_0030_dager,
            over_0060_dager: row.ledighet_over_0060_dager,
            over_0090_dager: row.ledighet_over_0090_dager,
            over_0180_dager: row.ledighet_over_0180_dager,
            over_0365_dager: row.ledighet_over_0365_dager,
            over_0730_dager: row.ledighet_over_0730_dager,
            over_1095_dager: row.ledighet_over_1095_dager,
        },
    }
}
