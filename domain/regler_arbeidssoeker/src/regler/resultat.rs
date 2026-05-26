use serde::{Deserialize, Serialize};

use super::regel_id::RegelId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProblemKind {
    SkalAvvises,
    MuligGrunnlagForAvvisning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub regel_id: RegelId,
    pub kind: ProblemKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrunnlagForGodkjenning {
    pub regel_id: RegelId,
}
