use super::oppdater::PdlDataOppdatering;
use anyhow::Result;
use chrono::Utc;
use tokio::{task::JoinHandle, time::sleep};

pub fn start_pdl_oppdatering_task(
    oppdatering: PdlDataOppdatering,
    intervall: std::time::Duration,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        loop {
            let data_tilgjengelig = oppdatering.kjoer_oppdatering(Utc::now()).await?;
            if !data_tilgjengelig {
                sleep(intervall).await;
            }
        }
    })
}
