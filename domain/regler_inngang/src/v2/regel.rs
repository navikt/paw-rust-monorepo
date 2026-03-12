use interne_hendelser::vo::Opplysning;
use super::betingelse::Betingelse;
use super::regel_id::RegelId;
use super::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Aksjon {
    GrunnlagForGodkjenning,
    SkalAvvises,
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
        Regel { id, betingelser, aksjon }
    }

    pub fn evaluer(&self, opplysninger: &[Opplysning]) -> bool {
        self.betingelser.iter().all(|b| b.eval(opplysninger))
    }

    pub fn ved_treff(&self, opplysninger: Vec<Opplysning>) -> Result<GrunnlagForGodkjenning, Problem> {
        match self.aksjon {
            Aksjon::GrunnlagForGodkjenning => Ok(GrunnlagForGodkjenning {
                opplysninger,
                regel_id: self.id.clone(),
            }),
            Aksjon::SkalAvvises => Err(Problem {
                opplysninger,
                regel_id: self.id.clone(),
                kind: ProblemKind::SkalAvvises,
            }),
            Aksjon::MuligGrunnlagForAvvisning => Err(Problem {
                opplysninger,
                regel_id: self.id.clone(),
                kind: ProblemKind::MuligGrunnlagForAvvisning,
            }),
        }
    }
}
