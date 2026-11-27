use crate::model::dao::tiltak_synk::BatchTiltakSynkRow;
use std::ops::ControlFlow;

pub mod task;
pub mod tiltak_synk;

#[derive(Debug, PartialEq)]
pub enum BatchSlutt {
    IngenVentende,
    Stoppet,
}

#[derive(Debug, Default, PartialEq)]
pub struct Kjoering {
    pub synkronisert: u32,
    pub feilet: u32,
    pub frigitt: u64,
    pub slutt: Option<BatchSlutt>,
}

pub trait BatchBehandler {
    fn behandle(
        &mut self,
        rows: Vec<BatchTiltakSynkRow>,
    ) -> impl Future<Output = ControlFlow<()>> + Send;
}
