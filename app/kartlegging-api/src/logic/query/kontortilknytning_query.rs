use crate::model::dao::kontortilknytning;
use crate::model::dto::kontortilknytning::{KontorType, Kontortilknytning};
use sqlx::{Postgres, Transaction};
use std::str::FromStr;

#[tracing::instrument(skip(tx))]
pub async fn finn_for_parent_id(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: i64,
) -> anyhow::Result<Vec<Kontortilknytning>> {
    tracing::info!("Henter tilknyttede kontorer for parent id");

    let rows = kontortilknytning::select_by_parent_id(tx, &parent_id).await?;
    let mut kontortilknytninger = Vec::new();
    for row in &rows {
        kontortilknytninger.push(Kontortilknytning {
            kontor_id: row.kontor_id.clone(),
            kontor_navn: row.kontor_navn.clone(),
            kontor_type: KontorType::from_str(row.kontor_type.as_str())?,
        });
    }
    Ok(kontortilknytninger)
}
