use crate::model::dao::kontortilknytning;
use crate::model::dto::kontortilknytning::{KontorType, Kontortilknytning};
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;
use std::str::FromStr;

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip_all)]
pub async fn finn_for_aktor_ider(
    tx: &mut Transaction<'_, Postgres>,
    aktor_ider: &[String],
) -> anyhow::Result<HashMap<String, Vec<Kontortilknytning>>> {
    tracing::info!("Henter tilknyttede kontorer for parent ider");

    let rows = kontortilknytning::select_by_aktor_ids(tx, aktor_ider).await?;
    let mut kontortilknytninger_by_aktor_id: HashMap<String, Vec<Kontortilknytning>> =
        HashMap::new();
    for row in &rows {
        kontortilknytninger_by_aktor_id
            .entry(row.aktor_id.clone())
            .or_default()
            .push(Kontortilknytning {
                kontor_id: row.kontor_id.clone(),
                kontor_navn: row.kontor_navn.clone(),
                kontor_type: KontorType::from_str(row.kontor_type.as_str())?,
            });
    }
    Ok(kontortilknytninger_by_aktor_id)
}
