use crate::model::dao::kontortilknytning;
use crate::model::dto::kontortilknytning::{KontorType, Kontortilknytning};
use sqlx::{Postgres, Transaction};
use std::str::FromStr;

#[tracing::instrument(skip(tx, aktor_id))]
pub async fn finn_for_aktor_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    aktor_id: &'a str,
) -> anyhow::Result<Vec<Kontortilknytning>> {
    tracing::info!("Henter tilknyttede kontorer for parent id");

    let rows = kontortilknytning::select_by_aktor_id(tx, aktor_id).await?;
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
