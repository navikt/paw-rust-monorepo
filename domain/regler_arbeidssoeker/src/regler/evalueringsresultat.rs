use serde::{Deserialize, Serialize};

use crate::regler::regel_id::RegelId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Evalueringsresultat {
    Godkjent { regel_ider: Vec<RegelId> },
    Avvist { regel_ider: Vec<RegelId> },
    KreverManuellVurdering { regel_ider: Vec<RegelId> },
}

impl Evalueringsresultat {
    pub fn er_godkjent(&self) -> bool {
        matches!(self, Self::Godkjent { .. })
    }

    pub fn er_avvist(&self) -> bool {
        matches!(self, Self::Avvist { .. })
    }

    pub fn krever_manuell_vurdering(&self) -> bool {
        matches!(self, Self::KreverManuellVurdering { .. })
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::Godkjent { .. } => "GODKJENT",
            Self::Avvist { .. } => "AVVIST",
            Self::KreverManuellVurdering { .. } => "KREVER_MANUELL_VURDERING",
        }
    }
}
