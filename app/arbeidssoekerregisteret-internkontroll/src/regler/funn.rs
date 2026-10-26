use uuid::Uuid;

use crate::regler::regel_katalog::RegelId;

#[derive(Debug, PartialEq, Eq)]
pub enum Funn {
    IngenTraceId,
    AltOk {
        regel_id: RegelId,
        regel_versjon: i32,
        trace_id: Uuid,
    },
    Feil(Detaljer),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Detaljer {
    pub regel_id: RegelId,
    pub regel_versjon: i32,
    pub trace_id: Uuid,
    pub utfall: i16,
}
