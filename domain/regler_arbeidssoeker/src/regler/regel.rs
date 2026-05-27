use crate::regler::evalueringsresultat::Evalueringsresultat;

use super::betingelse::Betingelse;
use super::regel_id::RegelId;
use interne_hendelser::vo::Opplysning;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Aksjon {
    SkalAvvises,
    GrunnlagForGodkjenning,
    MuligGrunnlagForAvvisning,
}

#[derive(Debug, Clone)]
pub struct Regel {
    pub id: RegelId,
    pub betingelser: Vec<Betingelse>,
    pub aksjon: Aksjon,
}

impl Regel {
    pub fn new(id: RegelId, betingelser: Vec<Betingelse>, aksjon: Aksjon) -> Self {
        Regel {
            id,
            betingelser,
            aksjon,
        }
    }

    pub fn evaluer(&self, opplysninger: &[Opplysning]) -> bool {
        self.betingelser.iter().all(|b| b.eval(opplysninger))
    }

    pub fn ved_treff(&self) -> Evalueringsresultat {
        match self.aksjon {
            Aksjon::GrunnlagForGodkjenning => Evalueringsresultat::Godkjent {
                regel_ider: vec![self.id.clone()],
            },
            Aksjon::SkalAvvises => Evalueringsresultat::Avvist {
                regel_ider: vec![self.id.clone()],
            },
            Aksjon::MuligGrunnlagForAvvisning => Evalueringsresultat::KreverManuellVurdering {
                regel_ider: vec![self.id.clone()],
            },
        }
    }
}
