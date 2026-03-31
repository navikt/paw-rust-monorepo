use crate::oppdater_pdl_data::PdlDataOppdatering;
use anyhow::Result;
use tokio::{task::JoinHandle, time::sleep};

pub fn start_pdl_oppdatering_task(
    oppdatering: PdlDataOppdatering,
    intervall: std::time::Duration,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        loop {
            oppdatering.kjoer_oppdatering().await?;
            sleep(intervall).await;
        }
    })
}
